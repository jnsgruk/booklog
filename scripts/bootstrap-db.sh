#!/usr/bin/env bash
set -euo pipefail

cargo build

# BOOKLOG_TOKEN must be set before running this script.
# To create a token:
#   1. Start the server: ./target/debug/booklog serve
#   2. Register at the URL printed on first start
#   3. Create a token: ./target/debug/booklog token create --name "bootstrap-token"
#   4. Export the token: export BOOKLOG_TOKEN=<token>
if [[ -z ${BOOKLOG_TOKEN:-} ]]; then
  echo "Error: BOOKLOG_TOKEN environment variable is not set."
  echo "Create a token first: ./target/debug/booklog token create --name bootstrap-token"
  exit 1
fi

BL=./target/debug/booklog

author_id() {
  $BL author list | jq -r ".[] | select(.name==\"$1\") | .id"
}

book_id() {
  $BL book list | jq -r ".[] | select(.title==\"$1\") | .id"
}

genre_id() {
  $BL genre list | jq -r ".[] | select(.name==\"$1\") | .id"
}

# ============================================================================
# Genres — 10 top-level genres for categorizing books
# ============================================================================

$BL genre add --name "Literary Fiction"
$BL genre add --name "Science Fiction"
$BL genre add --name "Fantasy"
$BL genre add --name "Mystery"
$BL genre add --name "Non-Fiction"
$BL genre add --name "Historical Fiction"
$BL genre add --name "Horror"
$BL genre add --name "Romance"
$BL genre add --name "Thriller"
$BL genre add --name "Biography"

# ============================================================================
# Authors — 89 authors from the book club list, spread over 7 months
# ============================================================================

# Month 1 (Aug 2025)

$BL author add --name "John Wyndham" --created-at "2025-08-01T08:00:00Z"
$BL author add --name "Charles Dickens" --created-at "2025-08-01T08:30:00Z"
$BL author add --name "Kathryn Stockett" --created-at "2025-08-02T09:00:00Z"
$BL author add --name "Jane Austen" --created-at "2025-08-03T10:00:00Z"
$BL author add --name "Michelle Obama" --created-at "2025-08-04T11:00:00Z"
$BL author add --name "Richard Osman" --created-at "2025-08-05T12:00:00Z"
$BL author add --name "Ruth Hogan" --created-at "2025-08-06T13:00:00Z"
$BL author add --name "Adam Kay" --created-at "2025-08-07T09:00:00Z"
$BL author add --name "Tony Hawks" --created-at "2025-08-08T10:00:00Z"
$BL author add --name "Agatha Christie" --created-at "2025-08-09T11:00:00Z"
$BL author add --name "Julia Quinn" --created-at "2025-08-10T12:00:00Z"
$BL author add --name "Douglas Adams" --created-at "2025-08-11T09:00:00Z"
$BL author add --name "Margaret Atwood" --created-at "2025-08-12T14:00:00Z"

# Month 2 (Sep 2025)

$BL author add --name "Chris Hadfield" --created-at "2025-09-01T08:00:00Z"
$BL author add --name "David Walliams" --created-at "2025-09-02T09:00:00Z"
$BL author add --name "A.M. Homes" --created-at "2025-09-03T10:00:00Z"
$BL author add --name "Jeffrey Deaver" --created-at "2025-09-04T11:00:00Z"
$BL author add --name "Bella Bathurst" --created-at "2025-09-05T12:00:00Z"
$BL author add --name "Kevin Kwan" --created-at "2025-09-06T13:00:00Z"
$BL author add --name "Stieg Larsson" --created-at "2025-09-08T09:00:00Z"
$BL author add --name "Tatton Spiller" --created-at "2025-09-09T10:00:00Z"
$BL author add --name "Nancy Springer" --created-at "2025-09-10T11:00:00Z"
$BL author add --name "Gail Honeyman" --created-at "2025-09-12T12:00:00Z"
$BL author add --name "Mark Haddon" --created-at "2025-09-14T09:00:00Z"
$BL author add --name "Edith Eva Eger" --created-at "2025-09-15T14:00:00Z"
$BL author add --name "Robert Thorogood" --created-at "2025-09-16T10:00:00Z"

# Month 3 (Oct 2025)

$BL author add --name "Vince Flynn" --created-at "2025-10-01T08:00:00Z"
$BL author add --name "Anthony Horowitz" --created-at "2025-10-02T09:00:00Z"
$BL author add --name "Caroline Criado Perez" --created-at "2025-10-03T10:00:00Z"
$BL author add --name "Arthur Conan Doyle" --created-at "2025-10-04T11:00:00Z"
$BL author add --name "Matt Haig" --created-at "2025-10-05T12:00:00Z"
$BL author add --name "David Attenborough" --created-at "2025-10-06T13:00:00Z"
$BL author add --name "Nora Roberts" --created-at "2025-10-07T09:00:00Z"
$BL author add --name "Neil Gaiman" --created-at "2025-10-08T10:00:00Z"
$BL author add --name "Chris and Rosie Ramsay" --created-at "2025-10-09T11:00:00Z"
$BL author add --name "Per Gahrton" --created-at "2025-10-10T12:00:00Z"
$BL author add --name "James Herriot" --created-at "2025-10-12T09:00:00Z"
$BL author add --name "Adam Hart-Davis" --created-at "2025-10-14T14:00:00Z"

# Month 4 (Nov 2025)

$BL author add --name "Katie Fforde" --created-at "2025-11-01T08:00:00Z"
$BL author add --name "Evie Dunmore" --created-at "2025-11-02T09:00:00Z"
$BL author add --name "Alex Pine" --created-at "2025-11-03T10:00:00Z"
$BL author add --name "Harper Lee" --created-at "2025-11-04T11:00:00Z"
$BL author add --name "Stanley Tucci" --created-at "2025-11-05T12:00:00Z"
$BL author add --name "James Comey" --created-at "2025-11-06T13:00:00Z"
$BL author add --name "Kate Morton" --created-at "2025-11-07T09:00:00Z"
$BL author add --name "Ben Elton" --created-at "2025-11-08T10:00:00Z"
$BL author add --name "Andy Weir" --created-at "2025-11-09T11:00:00Z"
$BL author add --name "Bella Mackie" --created-at "2025-11-10T12:00:00Z"
$BL author add --name "Annabel Abbs" --created-at "2025-11-12T09:00:00Z"
$BL author add --name "P.G. Wodehouse" --created-at "2025-11-14T14:00:00Z"

# Month 5 (Dec 2025)

$BL author add --name "Jo Nesbo" --created-at "2025-12-01T08:00:00Z"
$BL author add --name "Louise Gray" --created-at "2025-12-02T09:00:00Z"
$BL author add --name "Emily Henry" --created-at "2025-12-03T10:00:00Z"
$BL author add --name "Kate Adie" --created-at "2025-12-04T11:00:00Z"
$BL author add --name "Ben Macintyre" --created-at "2025-12-05T12:00:00Z"
$BL author add --name "Kate Jacobs" --created-at "2025-12-06T13:00:00Z"
$BL author add --name "David Hewson" --created-at "2025-12-07T09:00:00Z"
$BL author add --name "Helen Browning" --created-at "2025-12-08T10:00:00Z"
$BL author add --name "Jodi Picoult" --created-at "2025-12-09T11:00:00Z"
$BL author add --name "Janice Hallett" --created-at "2025-12-10T12:00:00Z"
$BL author add --name "Andrew Smith" --created-at "2025-12-12T09:00:00Z"
$BL author add --name "Peter Wohlleben" --created-at "2025-12-14T14:00:00Z"

# Month 6 (Jan 2026)

$BL author add --name "Jeffrey Archer" --created-at "2026-01-01T08:00:00Z"
$BL author add --name "Ian Fleming" --created-at "2026-01-02T09:00:00Z"
$BL author add --name "Graeme Macrae Burnet" --created-at "2026-01-03T10:00:00Z"
$BL author add --name "Khaled Hosseini" --created-at "2026-01-04T11:00:00Z"
$BL author add --name "Helen Fielding" --created-at "2026-01-05T12:00:00Z"
$BL author add --name "Claudia Hammond" --created-at "2026-01-06T13:00:00Z"
$BL author add --name "Jeremy Clarkson" --created-at "2026-01-07T09:00:00Z"
$BL author add --name "Gill Hornby" --created-at "2026-01-08T10:00:00Z"
$BL author add --name "Richard Coles" --created-at "2026-01-09T11:00:00Z"
$BL author add --name "Alexandra Benedict" --created-at "2026-01-10T12:00:00Z"
$BL author add --name "Billy Connolly" --created-at "2026-01-12T09:00:00Z"
$BL author add --name "Marie Benedict" --created-at "2026-01-14T14:00:00Z"

# Month 7 (Feb 2026)

$BL author add --name "Holly Jackson" --created-at "2026-02-01T08:00:00Z"
$BL author add --name "Gabrielle Zevin" --created-at "2026-02-01T09:00:00Z"
$BL author add --name "Drew Barrymore" --created-at "2026-02-02T10:00:00Z"
$BL author add --name "Richard Bach" --created-at "2026-02-02T11:00:00Z"
$BL author add --name "Bonnie Garmus" --created-at "2026-02-03T12:00:00Z"
$BL author add --name "Tim Marshall" --created-at "2026-02-03T13:00:00Z"
$BL author add --name "Tara Westover" --created-at "2026-02-04T09:00:00Z"
$BL author add --name "Alice Feeney" --created-at "2026-02-04T10:00:00Z"
$BL author add --name "Tess Gerritsen" --created-at "2026-02-05T11:00:00Z"
$BL author add --name "Anne Glenconner" --created-at "2026-02-05T12:00:00Z"
$BL author add --name "Janette Benaddi" --created-at "2026-02-06T09:00:00Z"
$BL author add --name "Natasha Tidd" --created-at "2026-02-06T14:00:00Z"
$BL author add --name "F.L. Everett" --created-at "2026-02-07T10:00:00Z"
$BL author add --name "David Baldacci" --created-at "2026-02-07T11:00:00Z"
$BL author add --name "Michael Crichton" --created-at "2026-02-08T12:00:00Z"

# ============================================================================
# Books — 100 books from the book club list
# ============================================================================

# Books 1-14 (Aug 2025)

$BL book add \
  --title "The Chrysalids" \
  --author-ids "$(author_id "John Wyndham")" \
  --isbn "978-0141032979" \
  --page-count 200 \
  --year-published 1955 \
  --publisher "Michael Joseph" \
  --language "English" \
  --primary-genre-id "$(genre_id "Science Fiction")" \
  --created-at "2025-08-01T09:00:00Z"

$BL book add \
  --title "A Christmas Carol" \
  --author-ids "$(author_id "Charles Dickens")" \
  --isbn "978-0141324524" \
  --page-count 128 \
  --year-published 1843 \
  --publisher "Chapman & Hall" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-08-02T09:00:00Z"

$BL book add \
  --title "The Help" \
  --author-ids "$(author_id "Kathryn Stockett")" \
  --isbn "978-0425232200" \
  --page-count 451 \
  --year-published 2009 \
  --publisher "Putnam" \
  --language "English" \
  --primary-genre-id "$(genre_id "Historical Fiction")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-08-03T09:00:00Z"

$BL book add \
  --title "Persuasion" \
  --author-ids "$(author_id "Jane Austen")" \
  --isbn "978-0141439686" \
  --page-count 249 \
  --year-published 1817 \
  --publisher "John Murray" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --secondary-genre-id "$(genre_id "Romance")" \
  --created-at "2025-08-04T09:00:00Z"

$BL book add \
  --title "Becoming" \
  --author-ids "$(author_id "Michelle Obama")" \
  --isbn "978-1524763138" \
  --page-count 448 \
  --year-published 2018 \
  --publisher "Crown" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --created-at "2025-08-05T09:00:00Z"

$BL book add \
  --title "The Thursday Murder Club" \
  --author-ids "$(author_id "Richard Osman")" \
  --isbn "978-0241425442" \
  --page-count 400 \
  --year-published 2020 \
  --publisher "Viking" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-08-06T09:00:00Z"

$BL book add \
  --title "The Keeper of Lost Things" \
  --author-ids "$(author_id "Ruth Hogan")" \
  --isbn "978-1473635487" \
  --page-count 289 \
  --year-published 2017 \
  --publisher "Two Roads" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-08-07T09:00:00Z"

$BL book add \
  --title "This is Going to Hurt" \
  --author-ids "$(author_id "Adam Kay")" \
  --isbn "978-1509858637" \
  --page-count 277 \
  --year-published 2017 \
  --publisher "Picador" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --description "Secret diaries of a junior doctor navigating the NHS." \
  --created-at "2025-08-08T09:00:00Z"

$BL book add \
  --title "Round Ireland with a Fridge" \
  --author-ids "$(author_id "Tony Hawks")" \
  --isbn "978-0091867379" \
  --page-count 247 \
  --year-published 1998 \
  --publisher "Ebury Press" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --created-at "2025-08-09T09:00:00Z"

$BL book add \
  --title "Murder at the Vicarage" \
  --author-ids "$(author_id "Agatha Christie")" \
  --isbn "978-0007120864" \
  --page-count 288 \
  --year-published 1930 \
  --publisher "Collins Crime Club" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-08-10T09:00:00Z"

$BL book add \
  --title "The Duke and I" \
  --author-ids "$(author_id "Julia Quinn")" \
  --isbn "978-0062353597" \
  --page-count 370 \
  --year-published 2000 \
  --publisher "Avon Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Romance")" \
  --secondary-genre-id "$(genre_id "Historical Fiction")" \
  --created-at "2025-08-11T09:00:00Z"

$BL book add \
  --title "The Hitchhiker's Guide to the Galaxy" \
  --author-ids "$(author_id "Douglas Adams")" \
  --isbn "978-0345391803" \
  --page-count 224 \
  --year-published 1979 \
  --publisher "Pan Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Science Fiction")" \
  --created-at "2025-08-12T09:00:00Z"

$BL book add \
  --title "The Handmaid's Tale" \
  --author-ids "$(author_id "Margaret Atwood")" \
  --isbn "978-0099740919" \
  --page-count 311 \
  --year-published 1985 \
  --publisher "McClelland and Stewart" \
  --language "English" \
  --primary-genre-id "$(genre_id "Science Fiction")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-08-13T09:00:00Z"

$BL book add \
  --title "An Astronaut's Guide to Life on Earth" \
  --author-ids "$(author_id "Chris Hadfield")" \
  --isbn "978-1447257103" \
  --page-count 304 \
  --year-published 2013 \
  --publisher "Macmillan" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --secondary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2025-08-14T09:00:00Z"

# Books 15-28 (Sep 2025)

$BL book add \
  --title "Gangsta Granny" \
  --author-ids "$(author_id "David Walliams")" \
  --isbn "978-0007371464" \
  --page-count 295 \
  --year-published 2011 \
  --publisher "HarperCollins" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-09-01T09:00:00Z"

$BL book add \
  --title "This Book Will Save Your Life" \
  --author-ids "$(author_id "A.M. Homes")" \
  --isbn "978-1862079885" \
  --page-count 372 \
  --year-published 2006 \
  --publisher "Granta Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-09-03T09:00:00Z"

$BL book add \
  --title "The Bodies Left Behind" \
  --author-ids "$(author_id "Jeffrey Deaver")" \
  --isbn "978-0340937228" \
  --page-count 386 \
  --year-published 2008 \
  --publisher "Hodder & Stoughton" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --secondary-genre-id "$(genre_id "Thriller")" \
  --created-at "2025-09-04T09:00:00Z"

$BL book add \
  --title "Field Work" \
  --author-ids "$(author_id "Bella Bathurst")" \
  --isbn "978-1788162517" \
  --page-count 320 \
  --year-published 2021 \
  --publisher "Profile Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --description "What land does to people and what people do to land." \
  --created-at "2025-09-05T09:00:00Z"

$BL book add \
  --title "Crazy Rich Asians" \
  --author-ids "$(author_id "Kevin Kwan")" \
  --isbn "978-1782393320" \
  --page-count 527 \
  --year-published 2013 \
  --publisher "Doubleday" \
  --language "English" \
  --primary-genre-id "$(genre_id "Romance")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-09-06T09:00:00Z"

$BL book add \
  --title "Great Expectations" \
  --author-ids "$(author_id "Charles Dickens")" \
  --isbn "978-0141439563" \
  --page-count 544 \
  --year-published 1861 \
  --publisher "Chapman & Hall" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-09-08T09:00:00Z"

$BL book add \
  --title "The Man Who Died Twice" \
  --author-ids "$(author_id "Richard Osman")" \
  --isbn "978-0241425428" \
  --page-count 432 \
  --year-published 2021 \
  --publisher "Viking" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-09-09T09:00:00Z"

$BL book add \
  --title "The Girl with the Dragon Tattoo" \
  --author-ids "$(author_id "Stieg Larsson")" \
  --isbn "978-1847245458" \
  --page-count 672 \
  --year-published 2005 \
  --publisher "Norstedts Forlag" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --secondary-genre-id "$(genre_id "Thriller")" \
  --created-at "2025-09-10T09:00:00Z"

$BL book add \
  --title "We're Living Through the Breakdown" \
  --author-ids "$(author_id "Tatton Spiller")" \
  --isbn "978-1783964970" \
  --page-count 256 \
  --year-published 2020 \
  --publisher "Elliott & Thompson" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2025-09-11T09:00:00Z"

$BL book add \
  --title "The Case of the Missing Marquess" \
  --author-ids "$(author_id "Nancy Springer")" \
  --isbn "978-0142409336" \
  --page-count 216 \
  --year-published 2006 \
  --publisher "Philomel Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --description "An Enola Holmes mystery." \
  --created-at "2025-09-12T09:00:00Z"

$BL book add \
  --title "Eleanor Oliphant Is Completely Fine" \
  --author-ids "$(author_id "Gail Honeyman")" \
  --isbn "978-0008172145" \
  --page-count 327 \
  --year-published 2017 \
  --publisher "HarperCollins" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-09-14T09:00:00Z"

$BL book add \
  --title "Hercule Poirot's Christmas" \
  --author-ids "$(author_id "Agatha Christie")" \
  --isbn "978-0007527540" \
  --page-count 272 \
  --year-published 1938 \
  --publisher "Collins Crime Club" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-09-15T09:00:00Z"

$BL book add \
  --title "The Curious Incident of the Dog in the Night-Time" \
  --author-ids "$(author_id "Mark Haddon")" \
  --isbn "978-0099450252" \
  --page-count 226 \
  --year-published 2003 \
  --publisher "Jonathan Cape" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-09-16T09:00:00Z"

# Books 28-42 (Oct 2025)

$BL book add \
  --title "The Choice" \
  --author-ids "$(author_id "Edith Eva Eger")" \
  --isbn "978-1846045127" \
  --page-count 384 \
  --year-published 2017 \
  --publisher "Rider" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --description "Embrace the possible. A Holocaust survivor's inspiring journey." \
  --created-at "2025-10-01T09:00:00Z"

$BL book add \
  --title "A Meditation on Murder" \
  --author-ids "$(author_id "Robert Thorogood")" \
  --isbn "978-1848453470" \
  --page-count 336 \
  --year-published 2015 \
  --publisher "Mira Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-10-02T09:00:00Z"

$BL book add \
  --title "Transfer of Power" \
  --author-ids "$(author_id "Vince Flynn")" \
  --isbn "978-1471176463" \
  --page-count 496 \
  --year-published 1999 \
  --publisher "Pocket Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Thriller")" \
  --created-at "2025-10-03T09:00:00Z"

$BL book add \
  --title "Stormbreaker" \
  --author-ids "$(author_id "Anthony Horowitz")" \
  --isbn "978-0142406113" \
  --page-count 234 \
  --year-published 2000 \
  --publisher "Walker Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Thriller")" \
  --created-at "2025-10-04T09:00:00Z"

$BL book add \
  --title "Invisible Women" \
  --author-ids "$(author_id "Caroline Criado Perez")" \
  --isbn "978-1784706289" \
  --page-count 432 \
  --year-published 2019 \
  --publisher "Chatto & Windus" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --description "Data bias in a world designed for men." \
  --created-at "2025-10-05T09:00:00Z"

$BL book add \
  --title "The Hound of the Baskervilles" \
  --author-ids "$(author_id "Arthur Conan Doyle")" \
  --isbn "978-0141199177" \
  --page-count 256 \
  --year-published 1902 \
  --publisher "George Newnes" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-10-06T09:00:00Z"

$BL book add \
  --title "Death on the Nile" \
  --author-ids "$(author_id "Agatha Christie")" \
  --isbn "978-0007119325" \
  --page-count 352 \
  --year-published 1937 \
  --publisher "Collins Crime Club" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-10-07T09:00:00Z"

$BL book add \
  --title "The Midnight Library" \
  --author-ids "$(author_id "Matt Haig")" \
  --isbn "978-1786892737" \
  --page-count 304 \
  --year-published 2020 \
  --publisher "Canongate" \
  --language "English" \
  --primary-genre-id "$(genre_id "Fantasy")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-10-08T09:00:00Z"

$BL book add \
  --title "Life on Air" \
  --author-ids "$(author_id "David Attenborough")" \
  --isbn "978-1849900010" \
  --page-count 416 \
  --year-published 2002 \
  --publisher "BBC Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --description "Memoirs of a broadcaster." \
  --created-at "2025-10-09T09:00:00Z"

$BL book add \
  --title "Shelter in Place" \
  --author-ids "$(author_id "Nora Roberts")" \
  --isbn "978-0349417820" \
  --page-count 432 \
  --year-published 2018 \
  --publisher "St. Martin's Press" \
  --language "English" \
  --primary-genre-id "$(genre_id "Thriller")" \
  --secondary-genre-id "$(genre_id "Romance")" \
  --created-at "2025-10-10T09:00:00Z"

$BL book add \
  --title "Stardust" \
  --author-ids "$(author_id "Neil Gaiman")" \
  --isbn "978-0747263340" \
  --page-count 248 \
  --year-published 1999 \
  --publisher "Headline" \
  --language "English" \
  --primary-genre-id "$(genre_id "Fantasy")" \
  --secondary-genre-id "$(genre_id "Romance")" \
  --created-at "2025-10-11T09:00:00Z"

$BL book add \
  --title "Shagged, Married, Annoyed" \
  --author-ids "$(author_id "Chris and Rosie Ramsay")" \
  --isbn "978-0241447147" \
  --page-count 304 \
  --year-published 2020 \
  --publisher "Michael Joseph" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --created-at "2025-10-12T09:00:00Z"

$BL book add \
  --title "Green Parties, Green Future" \
  --author-ids "$(author_id "Per Gahrton")" \
  --isbn "978-0745333397" \
  --page-count 208 \
  --year-published 2015 \
  --publisher "Pluto Press" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2025-10-13T09:00:00Z"

$BL book add \
  --title "The Bullet That Missed" \
  --author-ids "$(author_id "Richard Osman")" \
  --isbn "978-0241425435" \
  --page-count 400 \
  --year-published 2022 \
  --publisher "Viking" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-10-14T09:00:00Z"

$BL book add \
  --title "If Only They Could Talk" \
  --author-ids "$(author_id "James Herriot")" \
  --isbn "978-0330237819" \
  --page-count 207 \
  --year-published 1970 \
  --publisher "Michael Joseph" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-10-15T09:00:00Z"

# Books 43-56 (Nov 2025)

$BL book add \
  --title "Schrodinger's Cat" \
  --author-ids "$(author_id "Adam Hart-Davis")" \
  --isbn "978-1911130338" \
  --page-count 192 \
  --year-published 2018 \
  --publisher "Modern Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2025-11-01T09:00:00Z"

$BL book add \
  --title "Paradise Fields" \
  --author-ids "$(author_id "Katie Fforde")" \
  --isbn "978-1780896977" \
  --page-count 384 \
  --year-published 2003 \
  --publisher "Arrow" \
  --language "English" \
  --primary-genre-id "$(genre_id "Romance")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-11-02T09:00:00Z"

$BL book add \
  --title "A Rogue of One's Own" \
  --author-ids "$(author_id "Evie Dunmore")" \
  --isbn "978-0593098028" \
  --page-count 432 \
  --year-published 2020 \
  --publisher "Berkley" \
  --language "English" \
  --primary-genre-id "$(genre_id "Romance")" \
  --secondary-genre-id "$(genre_id "Historical Fiction")" \
  --created-at "2025-11-03T09:00:00Z"

$BL book add \
  --title "The Christmas Killer" \
  --author-ids "$(author_id "Alex Pine")" \
  --isbn "978-0008402174" \
  --page-count 368 \
  --year-published 2021 \
  --publisher "Avon" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --secondary-genre-id "$(genre_id "Thriller")" \
  --created-at "2025-11-04T09:00:00Z"

$BL book add \
  --title "To Kill a Mockingbird" \
  --author-ids "$(author_id "Harper Lee")" \
  --isbn "978-0099549482" \
  --page-count 309 \
  --year-published 1960 \
  --publisher "J. B. Lippincott" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-11-05T09:00:00Z"

$BL book add \
  --title "Taste" \
  --author-ids "$(author_id "Stanley Tucci")" \
  --isbn "978-0241500996" \
  --page-count 304 \
  --year-published 2021 \
  --publisher "Fig Tree" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --description "My life through food." \
  --created-at "2025-11-06T09:00:00Z"

$BL book add \
  --title "A Higher Loyalty" \
  --author-ids "$(author_id "James Comey")" \
  --isbn "978-1529000825" \
  --page-count 312 \
  --year-published 2018 \
  --publisher "Flatiron Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --secondary-genre-id "$(genre_id "Biography")" \
  --created-at "2025-11-07T09:00:00Z"

$BL book add \
  --title "The Lake House" \
  --author-ids "$(author_id "Kate Morton")" \
  --isbn "978-1447261223" \
  --page-count 512 \
  --year-published 2015 \
  --publisher "Pan Macmillan" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --secondary-genre-id "$(genre_id "Historical Fiction")" \
  --created-at "2025-11-08T09:00:00Z"

$BL book add \
  --title "Chart Throb" \
  --author-ids "$(author_id "Ben Elton")" \
  --isbn "978-0552773553" \
  --page-count 464 \
  --year-published 2006 \
  --publisher "Bantam Press" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-11-09T09:00:00Z"

$BL book add \
  --title "The Martian" \
  --author-ids "$(author_id "Andy Weir")" \
  --isbn "978-0091956141" \
  --page-count 369 \
  --year-published 2011 \
  --publisher "Crown" \
  --language "English" \
  --primary-genre-id "$(genre_id "Science Fiction")" \
  --secondary-genre-id "$(genre_id "Thriller")" \
  --created-at "2025-11-10T09:00:00Z"

$BL book add \
  --title "How to Kill Your Family" \
  --author-ids "$(author_id "Bella Mackie")" \
  --isbn "978-0008365943" \
  --page-count 374 \
  --year-published 2021 \
  --publisher "HarperCollins" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-11-11T09:00:00Z"

$BL book add \
  --title "The Language of Food" \
  --author-ids "$(author_id "Annabel Abbs")" \
  --isbn "978-1398504110" \
  --page-count 400 \
  --year-published 2022 \
  --publisher "Simon & Schuster" \
  --language "English" \
  --primary-genre-id "$(genre_id "Historical Fiction")" \
  --created-at "2025-11-13T09:00:00Z"

$BL book add \
  --title "My Man Jeeves" \
  --author-ids "$(author_id "P.G. Wodehouse")" \
  --isbn "978-0099513681" \
  --page-count 224 \
  --year-published 1919 \
  --publisher "George Newnes" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-11-14T09:00:00Z"

# Books 56-70 (Dec 2025)

$BL book add \
  --title "The Bat" \
  --author-ids "$(author_id "Jo Nesbo")" \
  --isbn "978-0099581864" \
  --page-count 372 \
  --year-published 1997 \
  --publisher "Aschehoug" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --secondary-genre-id "$(genre_id "Thriller")" \
  --created-at "2025-12-01T09:00:00Z"

$BL book add \
  --title "Avocado Anxiety" \
  --author-ids "$(author_id "Louise Gray")" \
  --isbn "978-1472966933" \
  --page-count 304 \
  --year-published 2022 \
  --publisher "Bloomsbury" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2025-12-02T09:00:00Z"

$BL book add \
  --title "Book Lovers" \
  --author-ids "$(author_id "Emily Henry")" \
  --isbn "978-0241995341" \
  --page-count 373 \
  --year-published 2022 \
  --publisher "Berkley" \
  --language "English" \
  --primary-genre-id "$(genre_id "Romance")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-12-03T09:00:00Z"

$BL book add \
  --title "The Kindness of Strangers" \
  --author-ids "$(author_id "Kate Adie")" \
  --isbn "978-0755310838" \
  --page-count 464 \
  --year-published 2002 \
  --publisher "Headline" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --created-at "2025-12-04T09:00:00Z"

$BL book add \
  --title "The Last Devil to Die" \
  --author-ids "$(author_id "Richard Osman")" \
  --isbn "978-0241425473" \
  --page-count 400 \
  --year-published 2023 \
  --publisher "Viking" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-12-05T09:00:00Z"

$BL book add \
  --title "Operation Mincemeat" \
  --author-ids "$(author_id "Ben Macintyre")" \
  --isbn "978-1408809211" \
  --page-count 432 \
  --year-published 2010 \
  --publisher "Bloomsbury" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2025-12-06T09:00:00Z"

$BL book add \
  --title "The Friday Night Knitting Club" \
  --author-ids "$(author_id "Kate Jacobs")" \
  --isbn "978-0340954140" \
  --page-count 352 \
  --year-published 2007 \
  --publisher "Hodder & Stoughton" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-12-07T09:00:00Z"

$BL book add \
  --title "The Sittaford Mystery" \
  --author-ids "$(author_id "Agatha Christie")" \
  --isbn "978-0007120956" \
  --page-count 256 \
  --year-published 1931 \
  --publisher "Collins Crime Club" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2025-12-08T09:00:00Z"

$BL book add \
  --title "A Season for the Dead" \
  --author-ids "$(author_id "David Hewson")" \
  --isbn "978-0330535830" \
  --page-count 486 \
  --year-published 2003 \
  --publisher "Macmillan" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --secondary-genre-id "$(genre_id "Thriller")" \
  --created-at "2025-12-09T09:00:00Z"

$BL book add \
  --title "Pig" \
  --author-ids "$(author_id "Helen Browning")" \
  --isbn "978-1472258052" \
  --page-count 256 \
  --year-published 2019 \
  --publisher "Bloomsbury" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2025-12-10T09:00:00Z"

$BL book add \
  --title "Wish You Were Here" \
  --author-ids "$(author_id "Jodi Picoult")" \
  --isbn "978-1473692435" \
  --page-count 352 \
  --year-published 2021 \
  --publisher "Hodder & Stoughton" \
  --language "English" \
  --primary-genre-id "$(genre_id "Romance")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2025-12-11T09:00:00Z"

$BL book add \
  --title "The Twyford Code" \
  --author-ids "$(author_id "Janice Hallett")" \
  --isbn "978-1788165624" \
  --page-count 432 \
  --year-published 2022 \
  --publisher "Viper" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --secondary-genre-id "$(genre_id "Thriller")" \
  --created-at "2025-12-12T09:00:00Z"

$BL book add \
  --title "Moondust" \
  --author-ids "$(author_id "Andrew Smith")" \
  --isbn "978-0747563426" \
  --page-count 372 \
  --year-published 2005 \
  --publisher "Bloomsbury" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2025-12-13T09:00:00Z"

$BL book add \
  --title "The Hidden Life of Trees" \
  --author-ids "$(author_id "Peter Wohlleben")" \
  --isbn "978-0008218430" \
  --page-count 288 \
  --year-published 2015 \
  --publisher "Greystone Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2025-12-14T09:00:00Z"

# Books 70-84 (Jan 2026)

$BL book add \
  --title "Shall We Tell the President?" \
  --author-ids "$(author_id "Jeffrey Archer")" \
  --isbn "978-0330518697" \
  --page-count 320 \
  --year-published 1977 \
  --publisher "Jonathan Cape" \
  --language "English" \
  --primary-genre-id "$(genre_id "Thriller")" \
  --secondary-genre-id "$(genre_id "Mystery")" \
  --created-at "2026-01-01T09:00:00Z"

$BL book add \
  --title "Casino Royale" \
  --author-ids "$(author_id "Ian Fleming")" \
  --isbn "978-0099575979" \
  --page-count 213 \
  --year-published 1953 \
  --publisher "Jonathan Cape" \
  --language "English" \
  --primary-genre-id "$(genre_id "Thriller")" \
  --created-at "2026-01-02T09:00:00Z"

$BL book add \
  --title "Case Study" \
  --author-ids "$(author_id "Graeme Macrae Burnet")" \
  --isbn "978-1913393779" \
  --page-count 288 \
  --year-published 2021 \
  --publisher "Saraband" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2026-01-03T09:00:00Z"

$BL book add \
  --title "SAS: Rogue Heroes" \
  --author-ids "$(author_id "Ben Macintyre")" \
  --isbn "978-0241186862" \
  --page-count 400 \
  --year-published 2016 \
  --publisher "Viking" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2026-01-04T09:00:00Z"

$BL book add \
  --title "A Thousand Splendid Suns" \
  --author-ids "$(author_id "Khaled Hosseini")" \
  --isbn "978-0747585893" \
  --page-count 372 \
  --year-published 2007 \
  --publisher "Bloomsbury" \
  --language "English" \
  --primary-genre-id "$(genre_id "Historical Fiction")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2026-01-05T09:00:00Z"

$BL book add \
  --title "We Solve Murders" \
  --author-ids "$(author_id "Richard Osman")" \
  --isbn "978-0241602096" \
  --page-count 432 \
  --year-published 2024 \
  --publisher "Viking" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2026-01-06T09:00:00Z"

$BL book add \
  --title "Bridget Jones's Diary" \
  --author-ids "$(author_id "Helen Fielding")" \
  --isbn "978-0330332774" \
  --page-count 310 \
  --year-published 1996 \
  --publisher "Picador" \
  --language "English" \
  --primary-genre-id "$(genre_id "Romance")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2026-01-07T09:00:00Z"

$BL book add \
  --title "The Art of Rest" \
  --author-ids "$(author_id "Claudia Hammond")" \
  --isbn "978-1786892829" \
  --page-count 320 \
  --year-published 2019 \
  --publisher "Canongate" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2026-01-08T09:00:00Z"

$BL book add \
  --title "Diddly Squat" \
  --author-ids "$(author_id "Jeremy Clarkson")" \
  --isbn "978-0241518021" \
  --page-count 256 \
  --year-published 2021 \
  --publisher "Michael Joseph" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --secondary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2026-01-09T09:00:00Z"

$BL book add \
  --title "Godmersham Park" \
  --author-ids "$(author_id "Gill Hornby")" \
  --isbn "978-1529904925" \
  --page-count 384 \
  --year-published 2022 \
  --publisher "Century" \
  --language "English" \
  --primary-genre-id "$(genre_id "Historical Fiction")" \
  --created-at "2026-01-10T09:00:00Z"

$BL book add \
  --title "Murder Before Evensong" \
  --author-ids "$(author_id "Richard Coles")" \
  --isbn "978-1474612630" \
  --page-count 352 \
  --year-published 2022 \
  --publisher "Weidenfeld & Nicolson" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2026-01-11T09:00:00Z"

$BL book add \
  --title "The Christmas Jigsaw Murders" \
  --author-ids "$(author_id "Alexandra Benedict")" \
  --isbn "978-1804183151" \
  --page-count 384 \
  --year-published 2023 \
  --publisher "Zaffre" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2026-01-12T09:00:00Z"

$BL book add \
  --title "Windswept and Interesting" \
  --author-ids "$(author_id "Billy Connolly")" \
  --isbn "978-1529318265" \
  --page-count 384 \
  --year-published 2021 \
  --publisher "Two Roads" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --created-at "2026-01-13T09:00:00Z"

$BL book add \
  --title "The Only Woman in the Room" \
  --author-ids "$(author_id "Marie Benedict")" \
  --isbn "978-1492666868" \
  --page-count 272 \
  --year-published 2019 \
  --publisher "Sourcebooks Landmark" \
  --language "English" \
  --primary-genre-id "$(genre_id "Historical Fiction")" \
  --created-at "2026-01-14T09:00:00Z"

# Books 84-100 (Feb 2026)

$BL book add \
  --title "A Good Girl's Guide to Murder" \
  --author-ids "$(author_id "Holly Jackson")" \
  --isbn "978-1405293181" \
  --page-count 400 \
  --year-published 2019 \
  --publisher "Egmont" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2026-02-01T09:00:00Z"

$BL book add \
  --title "Tomorrow, and Tomorrow, and Tomorrow" \
  --author-ids "$(author_id "Gabrielle Zevin")" \
  --isbn "978-1529115543" \
  --page-count 416 \
  --year-published 2022 \
  --publisher "Chatto & Windus" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2026-02-01T10:00:00Z"

$BL book add \
  --title "Wildflower" \
  --author-ids "$(author_id "Drew Barrymore")" \
  --isbn "978-0753557099" \
  --page-count 288 \
  --year-published 2015 \
  --publisher "Virgin Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --created-at "2026-02-02T09:00:00Z"

$BL book add \
  --title "Jonathan Livingston Seagull" \
  --author-ids "$(author_id "Richard Bach")" \
  --isbn "978-0006490340" \
  --page-count 127 \
  --year-published 1970 \
  --publisher "Macmillan" \
  --language "English" \
  --primary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2026-02-02T10:00:00Z"

$BL book add \
  --title "Project Hail Mary" \
  --author-ids "$(author_id "Andy Weir")" \
  --isbn "978-0593135204" \
  --page-count 476 \
  --year-published 2021 \
  --publisher "Ballantine Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Science Fiction")" \
  --secondary-genre-id "$(genre_id "Thriller")" \
  --created-at "2026-02-03T09:00:00Z"

$BL book add \
  --title "Lessons in Chemistry" \
  --author-ids "$(author_id "Bonnie Garmus")" \
  --isbn "978-0857528131" \
  --page-count 400 \
  --year-published 2022 \
  --publisher "Doubleday" \
  --language "English" \
  --primary-genre-id "$(genre_id "Historical Fiction")" \
  --secondary-genre-id "$(genre_id "Literary Fiction")" \
  --created-at "2026-02-03T10:00:00Z"

$BL book add \
  --title "Prisoners of Geography" \
  --author-ids "$(author_id "Tim Marshall")" \
  --isbn "978-1783961412" \
  --page-count 256 \
  --year-published 2015 \
  --publisher "Elliott & Thompson" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2026-02-04T09:00:00Z"

$BL book add \
  --title "Educated" \
  --author-ids "$(author_id "Tara Westover")" \
  --isbn "978-0099511021" \
  --page-count 352 \
  --year-published 2018 \
  --publisher "Random House" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --created-at "2026-02-04T10:00:00Z"

$BL book add \
  --title "Daisy Darker" \
  --author-ids "$(author_id "Alice Feeney")" \
  --isbn "978-1529024807" \
  --page-count 352 \
  --year-published 2022 \
  --publisher "Pan Macmillan" \
  --language "English" \
  --primary-genre-id "$(genre_id "Thriller")" \
  --secondary-genre-id "$(genre_id "Mystery")" \
  --created-at "2026-02-05T09:00:00Z"

$BL book add \
  --title "The Spy Coast" \
  --author-ids "$(author_id "Tess Gerritsen")" \
  --isbn "978-1662513992" \
  --page-count 334 \
  --year-published 2023 \
  --publisher "Thomas & Mercer" \
  --language "English" \
  --primary-genre-id "$(genre_id "Thriller")" \
  --created-at "2026-02-05T10:00:00Z"

$BL book add \
  --title "The Impossible Fortune" \
  --author-ids "$(author_id "Richard Osman")" \
  --isbn "978-0241743980" \
  --page-count 400 \
  --year-published 2025 \
  --publisher "Viking" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2026-02-06T09:00:00Z"

$BL book add \
  --title "Lady in Waiting" \
  --author-ids "$(author_id "Anne Glenconner")" \
  --isbn "978-1529328837" \
  --page-count 304 \
  --year-published 2019 \
  --publisher "Hodder & Stoughton" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --created-at "2026-02-06T10:00:00Z"

$BL book add \
  --title "Four Mums in a Boat" \
  --author-ids "$(author_id "Janette Benaddi")" \
  --isbn "978-0008241643" \
  --page-count 320 \
  --year-published 2018 \
  --publisher "HarperCollins" \
  --language "English" \
  --primary-genre-id "$(genre_id "Biography")" \
  --created-at "2026-02-07T09:00:00Z"

$BL book add \
  --title "A Short History of the World in 50 Lies" \
  --author-ids "$(author_id "Natasha Tidd")" \
  --isbn "978-1789294712" \
  --page-count 320 \
  --year-published 2023 \
  --publisher "Michael O'Mara" \
  --language "English" \
  --primary-genre-id "$(genre_id "Non-Fiction")" \
  --created-at "2026-02-07T10:00:00Z"

$BL book add \
  --title "Murder at Mistletoe Manor" \
  --author-ids "$(author_id "F.L. Everett")" \
  --isbn "978-0008532826" \
  --page-count 352 \
  --year-published 2022 \
  --publisher "HarperCollins" \
  --language "English" \
  --primary-genre-id "$(genre_id "Mystery")" \
  --created-at "2026-02-08T09:00:00Z"

$BL book add \
  --title "The Winner" \
  --author-ids "$(author_id "David Baldacci")" \
  --isbn "978-1538749784" \
  --page-count 544 \
  --year-published 1997 \
  --publisher "Warner Books" \
  --language "English" \
  --primary-genre-id "$(genre_id "Thriller")" \
  --created-at "2026-02-08T10:00:00Z"

$BL book add \
  --title "Jurassic Park" \
  --author-ids "$(author_id "Michael Crichton")" \
  --isbn "978-0099282914" \
  --page-count 448 \
  --year-published 1990 \
  --publisher "Alfred A. Knopf" \
  --language "English" \
  --primary-genre-id "$(genre_id "Science Fiction")" \
  --secondary-genre-id "$(genre_id "Thriller")" \
  --created-at "2026-02-09T09:00:00Z"

# ============================================================================
# Readings — 55 readings with varied statuses, ratings, formats, and reviews
# ============================================================================

# The Chrysalids — read (Aug 2025)
$BL reading add \
  --book-id "$(book_id "The Chrysalids")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-08-02" \
  --finished-at "2025-08-08" \
  --rating 3.5 \
  --quick-reviews "thought-provoking,slow-burn" \
  --created-at "2025-08-02T09:00:00Z"

# A Christmas Carol — read (Aug 2025)
$BL reading add \
  --book-id "$(book_id "A Christmas Carol")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-08-09" \
  --finished-at "2025-08-11" \
  --rating 4 \
  --quick-reviews "moving,quick-read" \
  --created-at "2025-08-09T10:00:00Z"

# The Help — read (Aug 2025)
$BL reading add \
  --book-id "$(book_id "The Help")" \
  --status "read" \
  --format "audiobook" \
  --started-at "2025-08-12" \
  --finished-at "2025-08-22" \
  --rating 5 \
  --quick-reviews "couldnt-put-down,great-characters" \
  --created-at "2025-08-12T09:00:00Z"

# Becoming — read (Aug 2025)
$BL reading add \
  --book-id "$(book_id "Becoming")" \
  --status "read" \
  --format "audiobook" \
  --started-at "2025-08-20" \
  --finished-at "2025-09-01" \
  --rating 5 \
  --quick-reviews "thought-provoking,moving" \
  --created-at "2025-08-20T11:00:00Z"

# The Thursday Murder Club — read (Aug 2025)
$BL reading add \
  --book-id "$(book_id "The Thursday Murder Club")" \
  --status "read" \
  --format "ereader" \
  --started-at "2025-08-25" \
  --finished-at "2025-09-02" \
  --rating 5 \
  --quick-reviews "loved-it,great-characters" \
  --created-at "2025-08-25T12:00:00Z"

# This is Going to Hurt — read (Sep 2025)
$BL reading add \
  --book-id "$(book_id "This is Going to Hurt")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-09-03" \
  --finished-at "2025-09-06" \
  --rating 5 \
  --quick-reviews "funny,moving" \
  --created-at "2025-09-03T09:00:00Z"

# The Hitchhiker's Guide to the Galaxy — read (Sep 2025)
$BL reading add \
  --book-id "$(book_id "The Hitchhiker's Guide to the Galaxy")" \
  --status "read" \
  --format "ereader" \
  --started-at "2025-09-08" \
  --finished-at "2025-09-12" \
  --rating 4.5 \
  --quick-reviews "funny,loved-it" \
  --created-at "2025-09-08T09:00:00Z"

# The Handmaid's Tale — read (Sep 2025)
$BL reading add \
  --book-id "$(book_id "The Handmaid's Tale")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-09-14" \
  --finished-at "2025-09-22" \
  --rating 4 \
  --quick-reviews "thought-provoking,couldnt-put-down" \
  --created-at "2025-09-14T09:00:00Z"

# Field Work — read (Sep 2025)
$BL reading add \
  --book-id "$(book_id "Field Work")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-09-24" \
  --finished-at "2025-10-02" \
  --rating 5 \
  --quick-reviews "beautiful-writing,thought-provoking" \
  --created-at "2025-09-24T12:00:00Z"

# Crazy Rich Asians — read (Sep 2025)
$BL reading add \
  --book-id "$(book_id "Crazy Rich Asians")" \
  --status "read" \
  --format "ereader" \
  --started-at "2025-09-28" \
  --finished-at "2025-10-05" \
  --rating 5 \
  --quick-reviews "page-turner,funny" \
  --created-at "2025-09-28T13:00:00Z"

# The Man Who Died Twice — read (Oct 2025)
$BL reading add \
  --book-id "$(book_id "The Man Who Died Twice")" \
  --status "read" \
  --format "ereader" \
  --started-at "2025-10-04" \
  --finished-at "2025-10-10" \
  --rating 5 \
  --quick-reviews "page-turner,well-plotted" \
  --created-at "2025-10-04T09:00:00Z"

# The Girl with the Dragon Tattoo — read (Oct 2025)
$BL reading add \
  --book-id "$(book_id "The Girl with the Dragon Tattoo")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-10-06" \
  --finished-at "2025-10-18" \
  --rating 4 \
  --quick-reviews "dense,page-turner" \
  --created-at "2025-10-06T09:00:00Z"

# The Curious Incident of the Dog in the Night-Time — read (Oct 2025)
$BL reading add \
  --book-id "$(book_id "The Curious Incident of the Dog in the Night-Time")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-10-12" \
  --finished-at "2025-10-16" \
  --rating 3.5 \
  --quick-reviews "great-characters,thought-provoking" \
  --created-at "2025-10-12T09:00:00Z"

# The Choice — read (Oct 2025)
$BL reading add \
  --book-id "$(book_id "The Choice")" \
  --status "read" \
  --format "audiobook" \
  --started-at "2025-10-18" \
  --finished-at "2025-10-26" \
  --rating 4 \
  --quick-reviews "moving,thought-provoking" \
  --created-at "2025-10-18T09:00:00Z"

# Transfer of Power — read (Oct 2025)
$BL reading add \
  --book-id "$(book_id "Transfer of Power")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-10-20" \
  --finished-at "2025-10-28" \
  --rating 4 \
  --quick-reviews "page-turner,couldnt-put-down" \
  --created-at "2025-10-20T08:00:00Z"

# Death on the Nile — read (Oct 2025)
$BL reading add \
  --book-id "$(book_id "Death on the Nile")" \
  --status "read" \
  --format "ereader" \
  --started-at "2025-10-25" \
  --finished-at "2025-10-29" \
  --rating 5 \
  --quick-reviews "well-plotted,loved-it" \
  --created-at "2025-10-25T14:00:00Z"

# Life on Air — read (Oct 2025)
$BL reading add \
  --book-id "$(book_id "Life on Air")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-10-28" \
  --finished-at "2025-11-06" \
  --rating 5 \
  --quick-reviews "thought-provoking,beautiful-writing" \
  --created-at "2025-10-28T13:00:00Z"

# Shelter in Place — read (Nov 2025)
$BL reading add \
  --book-id "$(book_id "Shelter in Place")" \
  --status "read" \
  --format "ereader" \
  --started-at "2025-11-02" \
  --finished-at "2025-11-10" \
  --rating 5 \
  --quick-reviews "page-turner,couldnt-put-down" \
  --created-at "2025-11-02T09:00:00Z"

# The Bullet That Missed — read (Nov 2025)
$BL reading add \
  --book-id "$(book_id "The Bullet That Missed")" \
  --status "read" \
  --format "audiobook" \
  --started-at "2025-11-08" \
  --finished-at "2025-11-14" \
  --rating 5 \
  --quick-reviews "funny,great-characters" \
  --created-at "2025-11-08T09:00:00Z"

# To Kill a Mockingbird — read (Nov 2025)
$BL reading add \
  --book-id "$(book_id "To Kill a Mockingbird")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-11-12" \
  --finished-at "2025-11-18" \
  --rating 2.5 \
  --created-at "2025-11-12T11:00:00Z"

# The Martian — read (Nov 2025)
$BL reading add \
  --book-id "$(book_id "The Martian")" \
  --status "read" \
  --format "ereader" \
  --started-at "2025-11-16" \
  --finished-at "2025-11-22" \
  --rating 5 \
  --quick-reviews "loved-it,page-turner" \
  --created-at "2025-11-16T11:00:00Z"

# A Higher Loyalty — read (Nov 2025)
$BL reading add \
  --book-id "$(book_id "A Higher Loyalty")" \
  --status "read" \
  --format "audiobook" \
  --started-at "2025-11-20" \
  --finished-at "2025-11-28" \
  --rating 4 \
  --created-at "2025-11-20T13:00:00Z"

# How to Kill Your Family — read (Nov 2025)
$BL reading add \
  --book-id "$(book_id "How to Kill Your Family")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-11-25" \
  --finished-at "2025-12-01" \
  --rating 3 \
  --created-at "2025-11-25T09:00:00Z"

# Avocado Anxiety — read (Dec 2025)
$BL reading add \
  --book-id "$(book_id "Avocado Anxiety")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-12-02" \
  --finished-at "2025-12-08" \
  --rating 5 \
  --quick-reviews "thought-provoking" \
  --created-at "2025-12-02T09:00:00Z"

# The Last Devil to Die — read (Dec 2025)
$BL reading add \
  --book-id "$(book_id "The Last Devil to Die")" \
  --status "read" \
  --format "ereader" \
  --started-at "2025-12-06" \
  --finished-at "2025-12-12" \
  --rating 5 \
  --quick-reviews "moving,beautiful-writing" \
  --created-at "2025-12-06T09:00:00Z"

# Operation Mincemeat — read (Dec 2025)
$BL reading add \
  --book-id "$(book_id "Operation Mincemeat")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-12-10" \
  --finished-at "2025-12-18" \
  --rating 5 \
  --quick-reviews "page-turner,thought-provoking" \
  --created-at "2025-12-10T12:00:00Z"

# Pig — read (Dec 2025)
$BL reading add \
  --book-id "$(book_id "Pig")" \
  --status "read" \
  --format "physical" \
  --started-at "2025-12-14" \
  --finished-at "2025-12-20" \
  --rating 5 \
  --quick-reviews "thought-provoking" \
  --created-at "2025-12-14T10:00:00Z"

# Wish You Were Here — read (Dec 2025)
$BL reading add \
  --book-id "$(book_id "Wish You Were Here")" \
  --status "read" \
  --format "ereader" \
  --started-at "2025-12-18" \
  --finished-at "2025-12-26" \
  --rating 4 \
  --created-at "2025-12-18T11:00:00Z"

# The Twyford Code — read (Dec 2025)
$BL reading add \
  --book-id "$(book_id "The Twyford Code")" \
  --status "read" \
  --format "audiobook" \
  --started-at "2025-12-22" \
  --finished-at "2025-12-30" \
  --rating 4 \
  --quick-reviews "well-plotted,page-turner" \
  --created-at "2025-12-22T12:00:00Z"

# SAS: Rogue Heroes — read (Jan 2026)
$BL reading add \
  --book-id "$(book_id "SAS: Rogue Heroes")" \
  --status "read" \
  --format "physical" \
  --started-at "2026-01-02" \
  --finished-at "2026-01-10" \
  --rating 5 \
  --quick-reviews "thought-provoking,great-characters" \
  --created-at "2026-01-02T09:00:00Z"

# A Thousand Splendid Suns — read (Jan 2026)
$BL reading add \
  --book-id "$(book_id "A Thousand Splendid Suns")" \
  --status "read" \
  --format "ereader" \
  --started-at "2026-01-06" \
  --finished-at "2026-01-14" \
  --rating 4 \
  --quick-reviews "moving,beautiful-writing" \
  --created-at "2026-01-06T11:00:00Z"

# We Solve Murders — read (Jan 2026)
$BL reading add \
  --book-id "$(book_id "We Solve Murders")" \
  --status "read" \
  --format "audiobook" \
  --started-at "2026-01-10" \
  --finished-at "2026-01-16" \
  --rating 5 \
  --quick-reviews "funny,page-turner" \
  --created-at "2026-01-10T09:00:00Z"

# Windswept and Interesting — read (Jan 2026)
$BL reading add \
  --book-id "$(book_id "Windswept and Interesting")" \
  --status "read" \
  --format "audiobook" \
  --started-at "2026-01-14" \
  --finished-at "2026-01-20" \
  --rating 4 \
  --quick-reviews "great-characters" \
  --created-at "2026-01-14T09:00:00Z"

# A Good Girl's Guide to Murder — read (Jan 2026)
$BL reading add \
  --book-id "$(book_id "A Good Girl's Guide to Murder")" \
  --status "read" \
  --format "ereader" \
  --started-at "2026-01-18" \
  --finished-at "2026-01-22" \
  --rating 4.5 \
  --quick-reviews "page-turner,well-plotted" \
  --created-at "2026-01-18T08:00:00Z"

# Project Hail Mary — read (Jan 2026)
$BL reading add \
  --book-id "$(book_id "Project Hail Mary")" \
  --status "read" \
  --format "physical" \
  --started-at "2026-01-20" \
  --finished-at "2026-01-28" \
  --rating 5 \
  --quick-reviews "loved-it,great-characters" \
  --created-at "2026-01-20T09:00:00Z"

# Lessons in Chemistry — read (Jan 2026)
$BL reading add \
  --book-id "$(book_id "Lessons in Chemistry")" \
  --status "read" \
  --format "ereader" \
  --started-at "2026-01-25" \
  --finished-at "2026-02-01" \
  --rating 5 \
  --quick-reviews "funny,great-characters" \
  --created-at "2026-01-25T10:00:00Z"

# Educated — read (Feb 2026)
$BL reading add \
  --book-id "$(book_id "Educated")" \
  --status "read" \
  --format "audiobook" \
  --started-at "2026-02-01" \
  --finished-at "2026-02-07" \
  --rating 4 \
  --quick-reviews "moving,thought-provoking" \
  --created-at "2026-02-01T09:00:00Z"

# The Spy Coast — read (Feb 2026)
$BL reading add \
  --book-id "$(book_id "The Spy Coast")" \
  --status "read" \
  --format "physical" \
  --started-at "2026-02-03" \
  --finished-at "2026-02-09" \
  --rating 4 \
  --quick-reviews "page-turner,couldnt-put-down" \
  --created-at "2026-02-03T11:00:00Z"

# The Impossible Fortune — reading, in progress (Feb 2026)
$BL reading add \
  --book-id "$(book_id "The Impossible Fortune")" \
  --status "reading" \
  --format "ereader" \
  --started-at "2026-02-06" \
  --created-at "2026-02-06T09:00:00Z"

# Lady in Waiting — reading, in progress (Feb 2026)
$BL reading add \
  --book-id "$(book_id "Lady in Waiting")" \
  --status "reading" \
  --format "physical" \
  --started-at "2026-02-07" \
  --created-at "2026-02-07T10:00:00Z"

# Murder at Mistletoe Manor — reading, in progress (Feb 2026)
$BL reading add \
  --book-id "$(book_id "Murder at Mistletoe Manor")" \
  --status "reading" \
  --format "ereader" \
  --started-at "2026-02-08" \
  --created-at "2026-02-08T09:00:00Z"

# On shelf (library, no reading started)

# Stormbreaker — on shelf
$BL user-book add --book-id "$(book_id "Stormbreaker")" --shelf "library"

# The Midnight Library — on shelf
$BL user-book add --book-id "$(book_id "The Midnight Library")" --shelf "library"

# Tomorrow, and Tomorrow, and Tomorrow — on shelf (book club)
$BL user-book add --book-id "$(book_id "Tomorrow, and Tomorrow, and Tomorrow")" --shelf "library" --book-club

# Casino Royale — on shelf
$BL user-book add --book-id "$(book_id "Casino Royale")" --shelf "library"

# Jurassic Park — wishlist (book club)
$BL user-book add --book-id "$(book_id "Jurassic Park")" --shelf "wishlist" --book-club

# The Winner — wishlist
$BL user-book add --book-id "$(book_id "The Winner")" --shelf "wishlist"

# A Short History of the World in 50 Lies — wishlist
$BL user-book add --book-id "$(book_id "A Short History of the World in 50 Lies")" --shelf "wishlist"

# Daisy Darker — wishlist (book club)
$BL user-book add --book-id "$(book_id "Daisy Darker")" --shelf "wishlist" --book-club

# Prisoners of Geography — wishlist (book club)
$BL user-book add --book-id "$(book_id "Prisoners of Geography")" --shelf "wishlist" --book-club

# Four Mums in a Boat — wishlist
$BL user-book add --book-id "$(book_id "Four Mums in a Boat")" --shelf "wishlist"

# Round Ireland with a Fridge — abandoned (Aug 2025)
$BL reading add \
  --book-id "$(book_id "Round Ireland with a Fridge")" \
  --status "abandoned" \
  --format "physical" \
  --started-at "2025-08-10" \
  --finished-at "2025-08-14" \
  --rating 2 \
  --quick-reviews "disappointing" \
  --created-at "2025-08-10T10:00:00Z"

# The Keeper of Lost Things — abandoned (Sep 2025)
$BL reading add \
  --book-id "$(book_id "The Keeper of Lost Things")" \
  --status "abandoned" \
  --format "ereader" \
  --started-at "2025-09-05" \
  --finished-at "2025-09-10" \
  --rating 2 \
  --quick-reviews "disappointing" \
  --created-at "2025-09-05T13:00:00Z"

# Green Parties, Green Future — abandoned (Oct 2025)
$BL reading add \
  --book-id "$(book_id "Green Parties, Green Future")" \
  --status "abandoned" \
  --format "physical" \
  --started-at "2025-10-14" \
  --finished-at "2025-10-17" \
  --rating 1.5 \
  --quick-reviews "disappointing" \
  --created-at "2025-10-14T09:00:00Z"

# Stardust — abandoned (Nov 2025)
$BL reading add \
  --book-id "$(book_id "Stardust")" \
  --status "abandoned" \
  --format "ereader" \
  --started-at "2025-11-05" \
  --finished-at "2025-11-09" \
  --rating 2 \
  --quick-reviews "disappointing" \
  --created-at "2025-11-05T10:00:00Z"

# ============================================================================
# Done — run scripts/fetch-images.py next to populate cover images
# ============================================================================

echo
echo "Bootstrapped database: 89 authors, 10 genres, 100 books, 49 readings, 4 on shelf, 6 wishlist entries"
echo
echo "Run 'python3 scripts/fetch-images.py' to fetch book covers and author photos from Open Library."
