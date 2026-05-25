#!/usr/bin/env python3
"""Import a Storygraph CSV export into booklog (with genre classification).

Usage:
    python3 scripts/import-storygraph-genres.py [CSV_PATH]

    CSV_PATH defaults to "dump.csv".
    Requires BOOKLOG_TOKEN environment variable to be set.

Optional environment variables:
    BOOKLOG_OPENROUTER_API_KEY  — enables LLM fallback for genre classification
    BOOKLOG_OPENROUTER_MODEL    — model to use (default: openrouter/free)
"""

import csv
import json
import math
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field

BL = "./target/debug/booklog"

FORMAT_MAP = {
    "paperback": "physical",
    "hardcover": "physical",
    "digital": "ereader",
    "audio": "audiobook",
}

STATUS_MAP = {
    "read": "read",
    "currently-reading": "reading",
    "did-not-finish": "abandoned",
}

GENRES = [
    "Literary Fiction",
    "Science Fiction",
    "Fantasy",
    "Mystery",
    "Non-Fiction",
    "Historical Fiction",
    "Horror",
    "Romance",
    "Thriller",
    "Biography",
]

# Map Open Library subject keywords (substring-matched) to genre names.
# Deliberately excludes bare "fiction" to avoid false matches against
# "science fiction", "historical fiction", etc. — handled as a fallback.
SUBJECT_GENRE_KEYWORDS = {
    "science fiction": "Science Fiction",
    "sci-fi": "Science Fiction",
    "fantasy": "Fantasy",
    "mystery": "Mystery",
    "detective": "Mystery",
    "thriller": "Thriller",
    "suspense": "Thriller",
    "horror": "Horror",
    "romance": "Romance",
    "love stories": "Romance",
    "biography": "Biography",
    "autobiography": "Biography",
    "memoir": "Biography",
    "memoirs": "Biography",
    "historical fiction": "Historical Fiction",
    "non-fiction": "Non-Fiction",
    "nonfiction": "Non-Fiction",
    "self-help": "Non-Fiction",
}

# Tiebreaker: specific genres before catch-alls.
GENRE_PRIORITY = [
    "Science Fiction",
    "Fantasy",
    "Mystery",
    "Thriller",
    "Horror",
    "Romance",
    "Historical Fiction",
    "Biography",
    "Non-Fiction",
    "Literary Fiction",
]

OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions"
OPENROUTER_DEFAULT_MODEL = "openrouter/free"


@dataclass
class ParsedRow:
    title: str
    authors: list
    isbn: str | None
    fmt: str | None
    status: str
    date_added: str | None
    dates_read: list
    rating: float | None


@dataclass
class ImportState:
    author_name_to_id: dict = field(default_factory=dict)
    book_key_to_id: dict = field(default_factory=dict)
    stats: dict = field(
        default_factory=lambda: {
            "authors": 0,
            "books": 0,
            "readings": 0,
            "user_books": 0,
            "skipped": 0,
            "errors": 0,
        }
    )


def run_cli(*args):
    """Run a booklog CLI command and return parsed JSON or None on failure."""
    cmd = [BL] + list(args)
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"  ERROR: {' '.join(cmd)}", file=sys.stderr)
        print(f"  {result.stderr.strip()}", file=sys.stderr)
        return None
    if result.stdout.strip():
        return json.loads(result.stdout)
    return None


def parse_partial_date(s):
    """Convert YYYY, YYYY/MM, or YYYY/MM/DD to YYYY-MM-DD."""
    s = s.strip()
    if not s:
        return None
    parts = s.split("/")
    if len(parts) == 1:
        return f"{parts[0]}-01-01"
    elif len(parts) == 2:
        return f"{parts[0]}-{parts[1].zfill(2)}-01"
    else:
        return f"{parts[0]}-{parts[1].zfill(2)}-{parts[2].zfill(2)}"


def parse_dates_read(dates_str):
    """Parse 'Dates Read' field into list of (start, end) tuples.

    Format examples:
      "2022/03/15-2022/04/01"              -> [("2022-03-15", "2022-04-01")]
      "2021/01-2021/02, 2022/05-2022/06"   -> [("2021-01-01", "2021-02-01"), ...]
      "2022-"                               -> [("2022-01-01", None)]
      ""                                    -> []
    """
    if not dates_str.strip():
        return []

    readings = []
    for chunk in dates_str.split(", "):
        chunk = chunk.strip()
        if not chunk:
            continue

        # The range separator is "-" between two date parts that use "/" internally.
        # Split on "-" but only the one separating start/end dates.
        # Dates are YYYY, YYYY/MM, or YYYY/MM/DD — none contain "-".
        parts = chunk.split("-", 1)
        start_raw = parts[0].strip() if parts[0].strip() else None
        end_raw = parts[1].strip() if len(parts) > 1 and parts[1].strip() else None

        start = parse_partial_date(start_raw) if start_raw else None
        end = parse_partial_date(end_raw) if end_raw else None
        readings.append((start, end))

    return readings


def round_rating(raw):
    """Round to nearest 0.5, rounding up on ties.

    3.75 -> 4.0, 4.25 -> 4.5, 2.75 -> 3.0, 1.75 -> 2.0
    """
    return math.floor(raw * 2 + 0.5) / 2


def is_valid_isbn(value):
    """Check if value looks like a real ISBN (not an ASIN)."""
    cleaned = value.replace("-", "").replace(" ", "")
    if cleaned.startswith(("978", "979")):
        return True
    if re.match(r"^\d{10}$", cleaned):
        return True
    if re.match(r"^\d{13}$", cleaned):
        return True
    return False


def parse_authors(authors_str):
    """Parse comma-separated author names from the Authors column.

    Deduplicates names (e.g. "Tatton Spiller, Tatton Spiller") while
    preserving order.
    """
    if not authors_str.strip():
        return []
    seen = set()
    result = []
    for name in authors_str.split(","):
        name = name.strip()
        if name and name not in seen:
            seen.add(name)
            result.append(name)
    return result


def format_created_at(date_added):
    """Convert YYYY/MM/DD from 'Date Added' column to RFC 3339 timestamp."""
    parts = date_added.strip().split("/")
    if len(parts) != 3:
        return None
    return f"{parts[0]}-{parts[1].zfill(2)}-{parts[2].zfill(2)}T00:00:00Z"


def parse_row(row):
    """Parse a single CSV row into a ParsedRow."""
    title = row["Title"].strip()
    authors = parse_authors(row.get("Authors", ""))

    raw_isbn = row.get("ISBN/UID", "").strip()
    isbn = raw_isbn if raw_isbn and is_valid_isbn(raw_isbn) else None

    raw_format = row.get("Format", "").strip().lower()
    fmt = FORMAT_MAP.get(raw_format)

    status = row.get("Read Status", "").strip().lower()

    raw_date_added = row.get("Date Added", "").strip()
    date_added = format_created_at(raw_date_added) if raw_date_added else None

    dates_read = parse_dates_read(row.get("Dates Read", ""))

    raw_rating = row.get("Star Rating", "").strip()
    if raw_rating:
        star_rating = float(raw_rating)
        rating = round_rating(star_rating) if star_rating > 0 else None
    else:
        rating = None

    return ParsedRow(
        title=title,
        authors=authors,
        isbn=isbn,
        fmt=fmt,
        status=status,
        date_added=date_added,
        dates_read=dates_read,
        rating=rating,
    )


def import_authors(rows, state):
    """Pass 1: collect unique author names and create them.

    Each author's created_at is set to the earliest date_added among all rows
    that include that author, so the timeline event appears at a sensible date.
    """
    author_earliest_date = {}
    for row in rows:
        for name in row.authors:
            if row.date_added:
                prev = author_earliest_date.get(name)
                if prev is None or row.date_added < prev:
                    author_earliest_date[name] = row.date_added
            elif name not in author_earliest_date:
                author_earliest_date[name] = None

    print(f"Creating {len(author_earliest_date)} authors...")
    for name in sorted(author_earliest_date):
        cmd = ["author", "add", "--name", name]
        earliest = author_earliest_date[name]
        if earliest:
            cmd.extend(["--created-at", earliest])
        result = run_cli(*cmd)
        if result:
            state.author_name_to_id[name] = result["id"]
            state.stats["authors"] += 1
            print(f"  + Author: {name} (id={result['id']})")
        else:
            state.stats["errors"] += 1
            print(f"  ! Failed to create author: {name}", file=sys.stderr)


def import_books(rows, state):
    """Pass 2: create books with author IDs.

    Deduplicates by (title, sorted_author_names) key so that duplicate rows
    like "Educated" (audio + paperback) create only one book entity.
    """
    print(f"\nCreating books...")
    for row in rows:
        book_key = (row.title, tuple(sorted(row.authors)))
        if book_key in state.book_key_to_id:
            print(f"  = Duplicate book, reusing: {row.title}")
            continue

        author_ids = []
        for name in row.authors:
            aid = state.author_name_to_id.get(name)
            if aid:
                author_ids.append(str(aid))
            else:
                print(
                    f"  ! Author ID not found for '{name}', "
                    f"skipping for book '{row.title}'",
                    file=sys.stderr,
                )

        cmd = ["book", "add", "--title", row.title]

        if author_ids:
            cmd.extend(["--author-ids", ",".join(author_ids)])

        if row.isbn:
            cmd.extend(["--isbn", row.isbn])

        if row.date_added:
            cmd.extend(["--created-at", row.date_added])

        result = run_cli(*cmd)
        if result:
            book_id = result["id"]
            state.book_key_to_id[book_key] = book_id
            state.stats["books"] += 1
            print(f"  + Book: {row.title} (id={book_id})")
        else:
            state.stats["errors"] += 1
            print(f"  ! Failed to create book: {row.title}", file=sys.stderr)


def import_readings(rows, state):
    """Pass 3: create readings and user-book entries.

    - "read"              -> reading with status=read
    - "currently-reading" -> reading with status=reading
    - "did-not-finish"    -> reading with status=abandoned
    - "to-read"           -> user-book on wishlist shelf (no reading)

    Multiple date ranges produce multiple reading records.
    Rating is applied only to the last reading.
    """
    print(f"\nCreating readings and shelf entries...")
    for row in rows:
        book_key = (row.title, tuple(sorted(row.authors)))
        book_id = state.book_key_to_id.get(book_key)

        if not book_id:
            print(f"  ! No book ID for '{row.title}', skipping", file=sys.stderr)
            state.stats["skipped"] += 1
            continue

        if row.status == "to-read":
            result = run_cli(
                "user-book", "add", "--book-id", str(book_id), "--shelf", "wishlist"
            )
            if result:
                state.stats["user_books"] += 1
                print(f"  + Wishlist: {row.title}")
            else:
                state.stats["errors"] += 1
            continue

        booklog_status = STATUS_MAP.get(row.status)
        if not booklog_status:
            print(
                f"  ! Unknown status '{row.status}' for '{row.title}', skipping",
                file=sys.stderr,
            )
            state.stats["skipped"] += 1
            continue

        readings_to_create = row.dates_read if row.dates_read else [(None, None)]

        for i, (start_date, end_date) in enumerate(readings_to_create):
            is_last = i == len(readings_to_create) - 1

            cmd = [
                "reading",
                "add",
                "--book-id",
                str(book_id),
                "--status",
                booklog_status,
            ]

            if row.fmt:
                cmd.extend(["--format", row.fmt])

            if start_date:
                cmd.extend(["--started-at", start_date])

            if end_date:
                cmd.extend(["--finished-at", end_date])

            if is_last and row.rating is not None:
                cmd.extend(["--rating", str(row.rating)])

            if row.date_added:
                cmd.extend(["--created-at", row.date_added])

            result = run_cli(*cmd)
            if result:
                state.stats["readings"] += 1
                suffix = (
                    f" ({i + 1}/{len(readings_to_create)})"
                    if len(readings_to_create) > 1
                    else ""
                )
                print(f"  + Reading: {row.title} [{booklog_status}]{suffix}")
            else:
                state.stats["errors"] += 1


# ---------------------------------------------------------------------------
# Pass 4: Genre classification
# ---------------------------------------------------------------------------


def ensure_genres_exist():
    """Create the standard genres if they don't already exist, return name -> id map."""
    existing = run_cli("genre", "list")
    genre_map = {g["name"]: g["id"] for g in existing} if existing else {}

    missing = [name for name in GENRES if name not in genre_map]
    if missing:
        print(f"Creating {len(missing)} genres...")
        for name in missing:
            result = run_cli("genre", "add", "--name", name)
            if result:
                genre_map[name] = result["id"]
                print(f"  + Genre: {name} (id={result['id']})")
            else:
                print(f"  ! Failed to create genre: {name}", file=sys.stderr)

    return genre_map


def lookup_open_library(isbn):
    """Query Open Library by ISBN and return subjects and page count.

    Returns {"subjects": [...], "page_count": int|None} or None on failure.
    """
    url = f"https://openlibrary.org/isbn/{isbn}.json"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "booklog-import/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode())
            # subjects can be strings or dicts with a "name" key
            raw = data.get("subjects", [])
            subjects = [s if isinstance(s, str) else s.get("name", "") for s in raw]
            page_count = data.get("number_of_pages")
            return {"subjects": subjects, "page_count": page_count}
    except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError, OSError):
        return None


def map_subjects_to_genres(subjects):
    """Map Open Library subjects to our genre list.

    Returns (primary_genre, secondary_genre) or (None, None) if no match.
    """
    genre_hits = {}
    for subject in subjects:
        subject_lower = subject.lower()
        for keyword, genre in SUBJECT_GENRE_KEYWORDS.items():
            if keyword in subject_lower:
                genre_hits[genre] = genre_hits.get(genre, 0) + 1

    # Fallback: if no specific genre matched, check for bare "fiction"
    if not genre_hits:
        for subject in subjects:
            sl = subject.lower().strip()
            if sl == "fiction" or "literary fiction" in sl:
                genre_hits["Literary Fiction"] = 1
                break

    if not genre_hits:
        return None, None

    sorted_genres = sorted(
        genre_hits.keys(),
        key=lambda g: (
            -genre_hits[g],
            GENRE_PRIORITY.index(g) if g in GENRE_PRIORITY else 99,
        ),
    )

    primary = sorted_genres[0]
    secondary = sorted_genres[1] if len(sorted_genres) > 1 and sorted_genres[1] != primary else None
    return primary, secondary


def extract_json_from_response(raw):
    """Extract a JSON object from a model response that may contain markdown fences."""
    raw = raw.strip()
    if raw.startswith("```json"):
        raw = raw[len("```json") :]
    if raw.startswith("```"):
        raw = raw[3:]
    if raw.endswith("```"):
        raw = raw[:-3]
    raw = raw.strip()

    start = raw.find("{")
    end = raw.rfind("}")
    if start != -1 and end != -1 and start < end:
        raw = raw[start : end + 1]

    return json.loads(raw)


def classify_with_llm(title, authors, api_key, model):
    """Use OpenRouter LLM to classify a book into genres.

    Returns (primary_genre, secondary_genre) or (None, None) on failure.
    """
    genres_str = ", ".join(GENRES)
    author_str = ", ".join(authors) if authors else "unknown"
    prompt = (
        f'Classify the book "{title}" by {author_str} into genres.\n\n'
        f"Available genres: {genres_str}\n\n"
        f"Return a JSON object with exactly these fields:\n"
        f'- "primary_genre": the single best-matching genre from the list above\n'
        f'- "secondary_genre": a secondary genre from the list above, or null\n\n'
        f"Return ONLY the JSON object, no other text."
    )

    payload = json.dumps(
        {
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": prompt}],
                }
            ],
        }
    ).encode()

    req = urllib.request.Request(
        OPENROUTER_URL,
        data=payload,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
            "User-Agent": "booklog-import/1.0",
        },
    )

    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read().decode())
            content = data["choices"][0]["message"]["content"]
            result = extract_json_from_response(content)
            primary = result.get("primary_genre")
            secondary = result.get("secondary_genre")
            if primary and primary not in GENRES:
                primary = None
            if secondary and secondary not in GENRES:
                secondary = None
            return primary, secondary
    except (
        urllib.error.URLError,
        urllib.error.HTTPError,
        json.JSONDecodeError,
        KeyError,
        IndexError,
        OSError,
    ):
        return None, None


def import_genres(rows, state):
    """Pass 4: classify books into genres using Open Library + LLM fallback."""
    genre_map = ensure_genres_exist()
    if not genre_map:
        print("\n  ! Failed to create genres, skipping classification", file=sys.stderr)
        return

    api_key = os.environ.get("BOOKLOG_OPENROUTER_API_KEY", "")
    model = os.environ.get("BOOKLOG_OPENROUTER_MODEL", OPENROUTER_DEFAULT_MODEL)
    llm_available = bool(api_key)

    if not llm_available:
        print("\n  (BOOKLOG_OPENROUTER_API_KEY not set; LLM fallback disabled)")

    # Deduplicate: only process each book once
    seen_book_ids = set()
    books_to_classify = []
    for row in rows:
        book_key = (row.title, tuple(sorted(row.authors)))
        book_id = state.book_key_to_id.get(book_key)
        if not book_id or book_id in seen_book_ids:
            continue
        seen_book_ids.add(book_id)
        books_to_classify.append((row, book_id))

    print(f"\nClassifying genres for {len(books_to_classify)} books...")

    ol_hits = 0
    llm_hits = 0
    failed = 0

    for row, book_id in books_to_classify:
        primary, secondary = None, None
        source = None
        page_count = None

        # Try Open Library first (if ISBN available)
        if row.isbn:
            time.sleep(1)  # rate limit: ~1 req/sec
            ol_data = lookup_open_library(row.isbn)
            if ol_data:
                page_count = ol_data["page_count"]
                if ol_data["subjects"]:
                    primary, secondary = map_subjects_to_genres(ol_data["subjects"])
                    if primary:
                        source = "OL"

        # Fallback to LLM
        if not primary and llm_available and row.authors:
            primary, secondary = classify_with_llm(
                row.title, row.authors, api_key, model
            )
            if primary:
                source = "LLM"

        if not primary and not page_count:
            print(f"  ? No genre found: {row.title}")
            failed += 1
            continue

        primary_id = genre_map.get(primary) if primary else None
        secondary_id = genre_map.get(secondary) if secondary else None

        if primary and not primary_id:
            print(f"  ? Unknown genre '{primary}' for: {row.title}")
            failed += 1
            continue

        cmd = ["book", "update", "--id", str(book_id)]
        if primary_id:
            cmd.extend(["--primary-genre-id", str(primary_id)])
        if secondary_id:
            cmd.extend(["--secondary-genre-id", str(secondary_id)])
        if page_count:
            cmd.extend(["--page-count", str(page_count)])

        result = run_cli(*cmd)
        if result:
            parts = []
            if primary:
                label = primary
                if secondary:
                    label += f" / {secondary}"
                parts.append(label)
            if page_count:
                parts.append(f"{page_count}pp")
            print(f"  + [{source or 'OL'}]: {row.title} -> {', '.join(parts)}")
            if source == "OL":
                ol_hits += 1
            elif source == "LLM":
                llm_hits += 1
        else:
            state.stats["errors"] += 1
            failed += 1

    print(f"\nGenre classification complete:")
    print(f"  Open Library: {ol_hits}")
    print(f"  LLM:          {llm_hits}")
    print(f"  Failed:       {failed}")


def main():
    csv_path = sys.argv[1] if len(sys.argv) > 1 else "dump.csv"

    if not os.environ.get("BOOKLOG_TOKEN"):
        print(
            "Error: BOOKLOG_TOKEN environment variable is not set.", file=sys.stderr
        )
        print(
            "Create a token first: "
            "./target/debug/booklog token create --name import-token",
            file=sys.stderr,
        )
        sys.exit(1)

    if not os.path.exists(csv_path):
        print(f"Error: CSV file not found: {csv_path}", file=sys.stderr)
        sys.exit(1)

    print("Building booklog...")
    result = subprocess.run(["cargo", "build"], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error: cargo build failed:\n{result.stderr}", file=sys.stderr)
        sys.exit(1)

    print(f"Reading {csv_path}...")
    rows = []
    with open(csv_path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for raw_row in reader:
            rows.append(parse_row(raw_row))

    print(f"Parsed {len(rows)} rows from CSV\n")

    state = ImportState()

    import_authors(rows, state)
    import_books(rows, state)
    import_readings(rows, state)
    import_genres(rows, state)

    s = state.stats
    print(f"\nImport complete:")
    print(f"  Authors:    {s['authors']}")
    print(f"  Books:      {s['books']}")
    print(f"  Readings:   {s['readings']}")
    print(f"  User books: {s['user_books']}")
    print(f"  Skipped:    {s['skipped']}")
    print(f"  Errors:     {s['errors']}")

    if s["errors"] > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
