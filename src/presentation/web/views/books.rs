use std::collections::HashSet;

use crate::domain::books::books::BookWithAuthors;
use crate::domain::books::readings::ReadingWithBook;
use crate::domain::formatting::format_rating;
use crate::domain::ids::BookId;
use crate::domain::user_books::{Shelf, UserBook};

use super::QuickReviewView;
use super::{author_path, book_path, format_author_label, genre_path, or_em_dash, reading_path};

pub struct BookLibraryInfo {
    pub date_added: String,
    pub status_label: String,
    pub format_label: String,
    pub book_club: bool,
    pub shelf_heading: &'static str,
    pub is_wishlist: bool,
}

impl BookLibraryInfo {
    pub fn from_domain(ub: &UserBook, reading: Option<&ReadingWithBook>, book_club: bool) -> Self {
        let is_wishlist = ub.shelf == Shelf::Wishlist;

        let (status_label, format_label) = if let Some(rwb) = reading {
            (
                rwb.reading.status.display_label().to_string(),
                or_em_dash(rwb.reading.format.map(|f| f.display_label())),
            )
        } else if is_wishlist {
            (
                "Wishlist".to_string(),
                crate::domain::formatting::EM_DASH.to_string(),
            )
        } else {
            (
                "On Shelf".to_string(),
                crate::domain::formatting::EM_DASH.to_string(),
            )
        };

        Self {
            date_added: ub.created_at.format("%Y-%m-%d").to_string(),
            status_label,
            format_label,
            book_club,
            shelf_heading: ub.shelf.display_label(),
            is_wishlist,
        }
    }
}

/// Card view for showing a reading on the book detail page.
pub struct BookReadingCardView {
    pub id: String,
    pub detail_path: String,
    pub status_label: String,
    pub format_label: String,
    pub rating: String,
    pub started_date: String,
    pub finished_date: String,
    pub created_date: String,
    pub quick_reviews: Vec<QuickReviewView>,
}

impl BookReadingCardView {
    pub fn from_domain(rwb: ReadingWithBook) -> Self {
        Self {
            id: rwb.reading.id.to_string(),
            detail_path: reading_path(rwb.reading.id),
            status_label: rwb.reading.status.display_label().to_string(),
            format_label: or_em_dash(rwb.reading.format.map(|f| f.display_label())),
            rating: or_em_dash(rwb.reading.rating.map(format_rating)),
            started_date: or_em_dash(rwb.reading.started_at),
            finished_date: or_em_dash(rwb.reading.finished_at),
            created_date: rwb.reading.created_at.format("%Y-%m-%d").to_string(),
            quick_reviews: rwb
                .reading
                .quick_reviews
                .iter()
                .copied()
                .map(QuickReviewView::from)
                .collect(),
        }
    }
}

pub struct BookView {
    pub id: String,
    pub detail_path: String,
    pub title: String,
    pub author_label: String,
    pub isbn: String,
    pub page_count: String,
    pub year_published: String,
    pub publisher: String,
    pub language: String,
    pub primary_genre: Option<String>,
    pub secondary_genre: Option<String>,
    pub primary_genre_id: Option<String>,
    pub secondary_genre_id: Option<String>,
    pub created_date: String,
    pub created_time: String,
    pub created_at_sort_key: i64,
    pub in_library: bool,
}

impl BookView {
    pub fn from_domain(
        book_with_authors: BookWithAuthors,
        library_book_ids: &HashSet<BookId>,
    ) -> Self {
        let book = book_with_authors.book;
        let in_library = library_book_ids.contains(&book.id);

        Self {
            id: book.id.to_string(),
            detail_path: book_path(book.id),
            title: book.title,
            author_label: format_author_label(&book_with_authors.authors),
            isbn: or_em_dash(book.isbn.as_deref()),
            page_count: or_em_dash(book.page_count),
            year_published: or_em_dash(book.year_published),
            publisher: or_em_dash(book.publisher.as_deref()),
            language: or_em_dash(book.language.as_deref()),
            primary_genre: book_with_authors.primary_genre,
            secondary_genre: book_with_authors.secondary_genre,
            primary_genre_id: book.primary_genre_id.map(|id| id.to_string()),
            secondary_genre_id: book.secondary_genre_id.map(|id| id.to_string()),
            created_date: book.created_at.format("%Y-%m-%d").to_string(),
            created_time: book.created_at.format("%H:%M").to_string(),
            created_at_sort_key: book.created_at.timestamp(),
            in_library,
        }
    }
}

pub struct BookDetailView {
    pub id: String,
    pub title: String,
    pub author_label: String,
    pub author_links: Vec<(String, String)>,
    pub isbn: String,
    pub description: Option<String>,
    pub page_count: String,
    pub year_published: String,
    pub publisher: String,
    pub language: String,
    pub genre_links: Vec<(String, String)>,
    pub created_date: String,
    pub created_time: String,
}

impl BookDetailView {
    pub fn from_domain(book_with_authors: BookWithAuthors) -> Self {
        let book = book_with_authors.book;
        let authors = &book_with_authors.authors;

        let author_links: Vec<(String, String)> = authors
            .iter()
            .map(|a| (a.author_id.to_string(), a.author_name.clone()))
            .collect();

        let mut genre_links = Vec::new();
        if let (Some(id), Some(name)) = (book.primary_genre_id, &book_with_authors.primary_genre) {
            genre_links.push((name.clone(), genre_path(id)));
        }
        if let (Some(id), Some(name)) =
            (book.secondary_genre_id, &book_with_authors.secondary_genre)
        {
            genre_links.push((name.clone(), genre_path(id)));
        }

        Self {
            id: book.id.to_string(),
            title: book.title,
            author_label: format_author_label(authors),
            author_links,
            isbn: or_em_dash(book.isbn.as_deref()),
            description: book.description,
            page_count: or_em_dash(book.page_count),
            year_published: or_em_dash(book.year_published),
            publisher: or_em_dash(book.publisher.as_deref()),
            language: or_em_dash(book.language.as_deref()),
            genre_links,
            created_date: book.created_at.format("%Y-%m-%d").to_string(),
            created_time: book.created_at.format("%H:%M").to_string(),
        }
    }
}

pub struct UserBookAuthorLink {
    pub name: String,
    pub path: String,
}

pub struct UserBookView {
    pub id: String,
    pub book_id: String,
    pub detail_path: String,
    pub title: String,
    pub author_label: String,
    pub author_links: Vec<UserBookAuthorLink>,
    pub page_count: String,
    pub year_published: String,
    pub created_date: String,
    pub created_time: String,
    pub created_at_sort_key: i64,
    pub relative_date_label: String,
    pub thumbnail_url: Option<String>,
    pub genre: String,
    pub book_club: bool,
    pub reading_status: String,
    pub reading_started: String,
    pub reading_finished: String,
}

impl UserBookView {
    pub fn from_domain(ubd: crate::domain::user_books::UserBookWithDetails) -> Self {
        let primary_genre = ubd.book.primary_genre.clone();
        let book = ubd.book.book;
        let authors = &ubd.book.authors;

        let author_links: Vec<UserBookAuthorLink> = authors
            .iter()
            .map(|a| UserBookAuthorLink {
                name: a.author_name.clone(),
                path: author_path(a.author_id),
            })
            .collect();

        let (reading_status, reading_started, reading_finished) =
            if let Some(ref rs) = ubd.reading_summary {
                (
                    rs.status.display_label().to_string(),
                    or_em_dash(rs.started_at.map(|d| d.format("%Y-%m-%d"))),
                    or_em_dash(rs.finished_at.map(|d| d.format("%Y-%m-%d"))),
                )
            } else {
                let em_dash = crate::domain::formatting::EM_DASH.to_string();
                ("On Shelf".to_string(), em_dash.clone(), em_dash)
            };

        Self {
            id: ubd.user_book.id.to_string(),
            book_id: book.id.to_string(),
            detail_path: book_path(book.id),
            title: book.title,
            author_label: format_author_label(authors),
            author_links,
            page_count: or_em_dash(book.page_count),
            year_published: or_em_dash(book.year_published),
            created_date: ubd.user_book.created_at.format("%Y-%m-%d").to_string(),
            created_time: ubd.user_book.created_at.format("%H:%M").to_string(),
            created_at_sort_key: ubd.user_book.created_at.timestamp(),
            relative_date_label: super::relative_date(ubd.user_book.created_at),
            thumbnail_url: None,
            genre: primary_genre.unwrap_or_else(|| crate::domain::formatting::EM_DASH.to_string()),
            book_club: ubd.user_book.book_club,
            reading_status,
            reading_started,
            reading_finished,
        }
    }
}

/// Lightweight card view for showing an author's books on the author detail page.
pub struct AuthorBookCardView {
    pub book_id: String,
    pub detail_path: String,
    pub title: String,
    pub page_count_label: String,
    pub thumbnail_url: Option<String>,
}

impl AuthorBookCardView {
    pub fn from_domain(bwa: BookWithAuthors) -> Self {
        let page_count_label = bwa
            .book
            .page_count
            .map(crate::domain::formatting::format_pages)
            .unwrap_or_default();
        Self {
            book_id: bwa.book.id.to_string(),
            detail_path: book_path(bwa.book.id),
            title: bwa.book.title,
            page_count_label,
            thumbnail_url: None,
        }
    }
}

pub struct BookOptionView {
    pub id: String,
    pub label: String,
    pub title: String,
    pub author_label: String,
}

impl From<BookWithAuthors> for BookOptionView {
    fn from(bwa: BookWithAuthors) -> Self {
        let author_label = format_author_label(&bwa.authors);
        Self {
            id: bwa.book.id.to_string(),
            label: format!("{author_label} - {}", bwa.book.title),
            title: bwa.book.title,
            author_label,
        }
    }
}
