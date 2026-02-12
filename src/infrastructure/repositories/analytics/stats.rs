use async_trait::async_trait;
use sqlx::{QueryBuilder, Row, query_as, query_scalar};

use crate::domain::RepositoryError;
use crate::domain::ids::UserId;
use crate::domain::repositories::StatsRepository;
use crate::domain::stats::{BookSummaryStats, CachedStats, ReadingStats};
use crate::infrastructure::database::{DatabaseDriver, DatabasePool};

// --- Internal record types ---

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct NameCount {
    name: String,
    count: i64,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct RatingCount {
    rating: f64,
    count: i64,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct MonthPages {
    name: String,
    pages: i64,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct TitlePages {
    title: String,
    page_count: i64,
}

// --- Helpers ---

fn db_err(err: sqlx::Error) -> RepositoryError {
    RepositoryError::unexpected(err.to_string())
}

fn name_counts(records: Vec<NameCount>) -> Vec<(String, u64)> {
    records
        .into_iter()
        .map(|r| (r.name, r.count as u64))
        .collect()
}

fn push_year_filter(qb: &mut QueryBuilder<'_, DatabaseDriver>, year: Option<i32>, column: &str) {
    if let Some(y) = year {
        qb.push(format!(" AND CAST(strftime('%Y', {column}) AS INTEGER) = "));
        qb.push_bind(y);
    }
}

struct RawBookStats {
    total_books: i64,
    total_authors: i64,
    genre_records: Vec<NameCount>,
    top_author: Option<String>,
    most_rated_author: Option<String>,
    most_rated_genre: Option<String>,
    page_distribution: Vec<NameCount>,
    year_distribution: Vec<NameCount>,
    top_authors_records: Vec<NameCount>,
    longest: Option<TitlePages>,
    shortest: Option<TitlePages>,
}

/// Processes raw book query results into the final stats struct.
fn build_book_summary(raw: RawBookStats) -> BookSummaryStats {
    let RawBookStats {
        total_books,
        total_authors,
        genre_records,
        top_author,
        most_rated_author,
        most_rated_genre,
        page_distribution,
        year_distribution,
        top_authors_records,
        longest,
        shortest,
    } = raw;
    let genre_counts = name_counts(genre_records);
    let unique_genres = genre_counts.len() as u64;
    let top_genre = genre_counts.first().map(|(name, _)| name.clone());
    let genre_counts: Vec<(String, u64)> = genre_counts.into_iter().collect();
    let max_genre_count = genre_counts.iter().map(|(_, c)| *c).max().unwrap_or(0);

    let page_count_distribution = name_counts(page_distribution);
    let year_published_distribution = name_counts(year_distribution);
    let max_year_published_count = year_published_distribution
        .iter()
        .map(|(_, c)| *c)
        .max()
        .unwrap_or(0);

    let top_authors = name_counts(top_authors_records);
    let max_top_author_count = top_authors.iter().map(|(_, c)| *c).max().unwrap_or(0);

    BookSummaryStats {
        total_books: total_books as u64,
        total_authors: total_authors as u64,
        unique_genres,
        top_genre,
        top_author,
        most_rated_author,
        most_rated_genre,
        genre_counts,
        max_genre_count,
        page_count_distribution,
        year_published_distribution,
        max_year_published_count,
        top_authors,
        max_top_author_count,
        longest_book: longest.map(|r| (r.title, r.page_count as i32)),
        shortest_book: shortest.map(|r| (r.title, r.page_count as i32)),
    }
}

// --- Repository ---

#[derive(Clone)]
pub struct SqlStatsRepository {
    pool: DatabasePool,
}

impl SqlStatsRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    // --- Book stat helpers ---

    async fn fetch_library_extremes(
        &self,
        uid: i64,
    ) -> Result<(Option<TitlePages>, Option<TitlePages>), RepositoryError> {
        let longest = query_as::<_, TitlePages>(
            r"SELECT b.title, b.page_count
               FROM user_books ub
               JOIN books b ON b.id = ub.book_id
               WHERE ub.user_id = ? AND ub.shelf = 'library'
                 AND b.page_count IS NOT NULL
               ORDER BY b.page_count DESC LIMIT 1",
        )
        .bind(uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        let shortest = query_as::<_, TitlePages>(
            r"SELECT b.title, b.page_count
               FROM user_books ub
               JOIN books b ON b.id = ub.book_id
               WHERE ub.user_id = ? AND ub.shelf = 'library'
                 AND b.page_count IS NOT NULL
               ORDER BY b.page_count ASC LIMIT 1",
        )
        .bind(uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        Ok((longest, shortest))
    }

    async fn fetch_year_extremes(
        &self,
        uid: i64,
        year: i32,
        cte: &str,
    ) -> Result<(Option<TitlePages>, Option<TitlePages>), RepositoryError> {
        let longest = query_as::<_, TitlePages>(&format!(
            "{cte} SELECT b.title, b.page_count \
             FROM year_books yb JOIN books b ON b.id = yb.book_id \
             WHERE b.page_count IS NOT NULL \
             ORDER BY b.page_count DESC LIMIT 1"
        ))
        .bind(uid)
        .bind(year)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        let shortest = query_as::<_, TitlePages>(&format!(
            "{cte} SELECT b.title, b.page_count \
             FROM year_books yb JOIN books b ON b.id = yb.book_id \
             WHERE b.page_count IS NOT NULL \
             ORDER BY b.page_count ASC LIMIT 1"
        ))
        .bind(uid)
        .bind(year)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        Ok((longest, shortest))
    }

    async fn fetch_library_distributions(
        &self,
        uid: i64,
    ) -> Result<(Vec<NameCount>, Vec<NameCount>), RepositoryError> {
        let page_distribution: Vec<NameCount> = query_as(
            r"SELECT
                 CASE
                   WHEN b.page_count < 200 THEN '< 200'
                   WHEN b.page_count <= 350 THEN '200 – 350'
                   WHEN b.page_count <= 500 THEN '350 – 500'
                   ELSE '500+'
                 END AS name,
                 COUNT(*) AS count
               FROM user_books ub
               JOIN books b ON b.id = ub.book_id
               WHERE ub.user_id = ? AND ub.shelf = 'library'
                 AND b.page_count IS NOT NULL
               GROUP BY name
               ORDER BY MIN(b.page_count)",
        )
        .bind(uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let year_distribution: Vec<NameCount> = query_as(
            r"SELECT
                 (b.year_published / 10 * 10) || 's' AS name,
                 COUNT(*) AS count
               FROM user_books ub
               JOIN books b ON b.id = ub.book_id
               WHERE ub.user_id = ? AND ub.shelf = 'library'
                 AND b.year_published IS NOT NULL
               GROUP BY b.year_published / 10
               ORDER BY count DESC",
        )
        .bind(uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok((page_distribution, year_distribution))
    }

    async fn fetch_year_distributions(
        &self,
        uid: i64,
        year: i32,
        cte: &str,
    ) -> Result<(Vec<NameCount>, Vec<NameCount>), RepositoryError> {
        let page_distribution: Vec<NameCount> = query_as(&format!(
            "{cte} SELECT \
               CASE \
                 WHEN b.page_count < 200 THEN '< 200' \
                 WHEN b.page_count <= 350 THEN '200 – 350' \
                 WHEN b.page_count <= 500 THEN '350 – 500' \
                 ELSE '500+' \
               END AS name, \
               COUNT(*) AS count \
             FROM year_books yb \
             JOIN books b ON b.id = yb.book_id \
             WHERE b.page_count IS NOT NULL \
             GROUP BY name ORDER BY MIN(b.page_count)"
        ))
        .bind(uid)
        .bind(year)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let year_distribution: Vec<NameCount> = query_as(&format!(
            "{cte} SELECT (b.year_published / 10 * 10) || 's' AS name, COUNT(*) AS count \
             FROM year_books yb JOIN books b ON b.id = yb.book_id \
             WHERE b.year_published IS NOT NULL \
             GROUP BY b.year_published / 10 ORDER BY count DESC"
        ))
        .bind(uid)
        .bind(year)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok((page_distribution, year_distribution))
    }

    // --- Most-rated queries ---

    async fn fetch_most_rated_author(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<Option<String>, RepositoryError> {
        let mut qb = QueryBuilder::new(
            r"SELECT a.name AS name, CAST(SUM(r.rating) AS INTEGER) AS count
               FROM readings r
               JOIN book_authors ba ON r.book_id = ba.book_id AND ba.role = 'author'
               JOIN authors a ON ba.author_id = a.id
               WHERE r.user_id = ",
        );
        qb.push_bind(uid);
        qb.push(" AND r.status = 'read' AND r.rating IS NOT NULL");
        push_year_filter(&mut qb, year, "r.finished_at");
        qb.push(" GROUP BY a.id ORDER BY SUM(r.rating) DESC LIMIT 1");
        let record: Option<NameCount> = qb
            .build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(record.map(|r| r.name))
    }

    async fn fetch_most_rated_genre(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<Option<String>, RepositoryError> {
        let mut qb = QueryBuilder::new(
            r"SELECT g.name AS name, CAST(SUM(r.rating) AS INTEGER) AS count
               FROM readings r
               JOIN books b ON r.book_id = b.id
               JOIN genres g ON g.id IN (b.primary_genre_id, b.secondary_genre_id)
               WHERE r.user_id = ",
        );
        qb.push_bind(uid);
        qb.push(" AND r.status = 'read' AND r.rating IS NOT NULL AND g.id IS NOT NULL");
        push_year_filter(&mut qb, year, "r.finished_at");
        qb.push(" GROUP BY g.id ORDER BY SUM(r.rating) DESC LIMIT 1");
        let record: Option<NameCount> = qb
            .build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(record.map(|r| r.name))
    }

    // --- Reading stat queries (shared between all-time and per-year) ---

    async fn fetch_books_read(&self, uid: i64, year: Option<i32>) -> Result<i64, RepositoryError> {
        let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM readings WHERE user_id = ");
        qb.push_bind(uid);
        qb.push(" AND status = 'read'");
        push_year_filter(&mut qb, year, "finished_at");
        let (count,): (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(count)
    }

    async fn fetch_pages_read(&self, uid: i64, year: Option<i32>) -> Result<i64, RepositoryError> {
        let mut qb = QueryBuilder::new(
            "SELECT COALESCE(SUM(bk.page_count), 0) FROM readings r \
             JOIN books bk ON r.book_id = bk.id WHERE r.user_id = ",
        );
        qb.push_bind(uid);
        qb.push(" AND r.status = 'read' AND bk.page_count IS NOT NULL");
        push_year_filter(&mut qb, year, "r.finished_at");
        let (pages,): (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(pages)
    }

    async fn fetch_average_rating(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<Option<f64>, RepositoryError> {
        let mut qb = QueryBuilder::new("SELECT AVG(rating) FROM readings WHERE user_id = ");
        qb.push_bind(uid);
        qb.push(" AND rating IS NOT NULL AND status = 'read'");
        push_year_filter(&mut qb, year, "finished_at");
        let (avg,): (Option<f64>,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(avg)
    }

    async fn fetch_average_days_to_finish(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<Option<f64>, RepositoryError> {
        let mut qb = QueryBuilder::new(
            "SELECT AVG(julianday(finished_at) - julianday(started_at)) \
             FROM readings WHERE user_id = ",
        );
        qb.push_bind(uid);
        qb.push(" AND status = 'read' AND started_at IS NOT NULL AND finished_at IS NOT NULL");
        push_year_filter(&mut qb, year, "finished_at");
        let (avg,): (Option<f64>,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(avg)
    }

    async fn fetch_rating_distribution(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<Vec<(f64, u64)>, RepositoryError> {
        let mut qb =
            QueryBuilder::new("SELECT rating, COUNT(*) AS count FROM readings WHERE user_id = ");
        qb.push_bind(uid);
        qb.push(" AND rating IS NOT NULL AND status = 'read'");
        push_year_filter(&mut qb, year, "finished_at");
        qb.push(" GROUP BY rating ORDER BY rating");
        let records: Vec<RatingCount> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(records
            .into_iter()
            .map(|r| (r.rating, r.count as u64))
            .collect())
    }

    async fn fetch_monthly_books(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<Vec<(String, u64)>, RepositoryError> {
        let mut qb = QueryBuilder::new(
            r"WITH months(m) AS (
                 VALUES (1),(2),(3),(4),(5),(6),(7),(8),(9),(10),(11),(12)
               )
               SELECT
                 CASE m
                   WHEN 1 THEN 'Jan' WHEN 2 THEN 'Feb' WHEN 3 THEN 'Mar'
                   WHEN 4 THEN 'Apr' WHEN 5 THEN 'May' WHEN 6 THEN 'Jun'
                   WHEN 7 THEN 'Jul' WHEN 8 THEN 'Aug' WHEN 9 THEN 'Sep'
                   WHEN 10 THEN 'Oct' WHEN 11 THEN 'Nov' WHEN 12 THEN 'Dec'
                 END AS name,
                 COUNT(r.id) AS count
               FROM months
               LEFT JOIN readings r
                 ON CAST(strftime('%m', r.finished_at) AS INTEGER) = m
                 AND ",
        );
        match year {
            Some(y) => {
                qb.push("strftime('%Y', r.finished_at) = ");
                qb.push_bind(y.to_string());
            }
            None => {
                qb.push("strftime('%Y', r.finished_at) = strftime('%Y', 'now')");
            }
        }
        qb.push(" AND r.status = 'read' AND r.user_id = ");
        qb.push_bind(uid);
        qb.push(" GROUP BY m ORDER BY m");
        let records: Vec<NameCount> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(name_counts(records))
    }

    async fn fetch_monthly_pages(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<Vec<(String, i64)>, RepositoryError> {
        let mut qb = QueryBuilder::new(
            r"WITH months(m) AS (
                 VALUES (1),(2),(3),(4),(5),(6),(7),(8),(9),(10),(11),(12)
               )
               SELECT
                 CASE m
                   WHEN 1 THEN 'Jan' WHEN 2 THEN 'Feb' WHEN 3 THEN 'Mar'
                   WHEN 4 THEN 'Apr' WHEN 5 THEN 'May' WHEN 6 THEN 'Jun'
                   WHEN 7 THEN 'Jul' WHEN 8 THEN 'Aug' WHEN 9 THEN 'Sep'
                   WHEN 10 THEN 'Oct' WHEN 11 THEN 'Nov' WHEN 12 THEN 'Dec'
                 END AS name,
                 COALESCE(SUM(bk.page_count), 0) AS pages
               FROM months
               LEFT JOIN readings r
                 ON CAST(strftime('%m', r.finished_at) AS INTEGER) = m
                 AND ",
        );
        match year {
            Some(y) => {
                qb.push("strftime('%Y', r.finished_at) = ");
                qb.push_bind(y.to_string());
            }
            None => {
                qb.push("strftime('%Y', r.finished_at) = strftime('%Y', 'now')");
            }
        }
        qb.push(" AND r.status = 'read' AND r.user_id = ");
        qb.push_bind(uid);
        qb.push(
            " LEFT JOIN books bk ON r.book_id = bk.id AND bk.page_count IS NOT NULL \
             GROUP BY m ORDER BY m",
        );
        let records: Vec<MonthPages> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(records.into_iter().map(|r| (r.name, r.pages)).collect())
    }

    async fn fetch_pace_distribution(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<Vec<(String, u64)>, RepositoryError> {
        let mut qb = QueryBuilder::new(
            r"SELECT pace AS name, COUNT(*) AS count
               FROM (
                 SELECT
                   CASE
                     WHEN bk.page_count * 1.0 / MAX(1, julianday(r.finished_at) - julianday(r.started_at)) < 15 THEN 'Slow'
                     WHEN bk.page_count * 1.0 / MAX(1, julianday(r.finished_at) - julianday(r.started_at)) <= 40 THEN 'Medium'
                     ELSE 'Fast'
                   END AS pace
                 FROM readings r
                 JOIN books bk ON r.book_id = bk.id
                 WHERE r.user_id = ",
        );
        qb.push_bind(uid);
        qb.push(
            " AND r.status = 'read' \
             AND r.started_at IS NOT NULL AND r.finished_at IS NOT NULL \
             AND bk.page_count IS NOT NULL \
             AND julianday(r.finished_at) >= julianday(r.started_at)",
        );
        push_year_filter(&mut qb, year, "r.finished_at");
        qb.push(
            ") GROUP BY pace \
             ORDER BY CASE pace WHEN 'Slow' THEN 1 WHEN 'Medium' THEN 2 ELSE 3 END",
        );
        let records: Vec<NameCount> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(name_counts(records))
    }

    async fn fetch_format_counts(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<Vec<(String, u64)>, RepositoryError> {
        let mut qb = QueryBuilder::new(
            r"SELECT
                 CASE format
                   WHEN 'physical' THEN 'Physical'
                   WHEN 'ereader' THEN 'eReader'
                   WHEN 'audiobook' THEN 'Audiobook'
                 END AS name,
                 COUNT(*) AS count
               FROM readings
               WHERE user_id = ",
        );
        qb.push_bind(uid);
        qb.push(" AND format IS NOT NULL AND status = 'read'");
        push_year_filter(&mut qb, year, "finished_at");
        qb.push(" GROUP BY format ORDER BY count DESC");
        let records: Vec<NameCount> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(name_counts(records))
    }

    async fn fetch_books_abandoned(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<i64, RepositoryError> {
        let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM readings WHERE user_id = ");
        qb.push_bind(uid);
        qb.push(" AND status = 'abandoned'");
        push_year_filter(&mut qb, year, "started_at");
        let (count,): (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(count)
    }

    // --- Yearly queries (all-time view) ---

    async fn fetch_yearly_books(&self, uid: i64) -> Result<Vec<(String, u64)>, RepositoryError> {
        let records: Vec<NameCount> = query_as(
            r"SELECT strftime('%Y', finished_at) AS name, COUNT(*) AS count
               FROM readings
               WHERE user_id = ? AND status = 'read' AND finished_at IS NOT NULL
               GROUP BY name ORDER BY name",
        )
        .bind(uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(name_counts(records))
    }

    async fn fetch_yearly_pages(&self, uid: i64) -> Result<Vec<(String, i64)>, RepositoryError> {
        let records: Vec<MonthPages> = query_as(
            r"SELECT strftime('%Y', r.finished_at) AS name,
                     COALESCE(SUM(bk.page_count), 0) AS pages
               FROM readings r
               JOIN books bk ON r.book_id = bk.id
               WHERE r.user_id = ? AND r.status = 'read' AND r.finished_at IS NOT NULL
                 AND bk.page_count IS NOT NULL
               GROUP BY name ORDER BY name",
        )
        .bind(uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(records.into_iter().map(|r| (r.name, r.pages)).collect())
    }

    // --- Reading stats assembly ---

    async fn build_reading_stats(
        &self,
        uid: i64,
        year: Option<i32>,
    ) -> Result<ReadingStats, RepositoryError> {
        let books_all_time = self.fetch_books_read(uid, year).await? as u64;
        let pages_all_time = self.fetch_pages_read(uid, year).await?;
        let average_rating = self.fetch_average_rating(uid, year).await?;
        let average_days_to_finish = self.fetch_average_days_to_finish(uid, year).await?;
        let rating_distribution = self.fetch_rating_distribution(uid, year).await?;
        let max_rating_count = rating_distribution
            .iter()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(0);
        let monthly_books = self.fetch_monthly_books(uid, year).await?;
        let max_monthly_books = monthly_books.iter().map(|(_, c)| *c).max().unwrap_or(0);
        let monthly_pages = self.fetch_monthly_pages(uid, year).await?;
        let max_monthly_pages = monthly_pages.iter().map(|(_, p)| *p).max().unwrap_or(0);
        let pace_distribution = self.fetch_pace_distribution(uid, year).await?;
        let format_counts = self.fetch_format_counts(uid, year).await?;
        let books_abandoned = self.fetch_books_abandoned(uid, year).await? as u64;

        // Yearly aggregation only for the all-time view
        let (yearly_books, yearly_pages, max_yearly_books, max_yearly_pages) = if year.is_none() {
            let yb = self.fetch_yearly_books(uid).await?;
            let yp = self.fetch_yearly_pages(uid).await?;
            let mb = yb.iter().map(|(_, c)| *c).max().unwrap_or(0);
            let mp = yp.iter().map(|(_, p)| *p).max().unwrap_or(0);
            (yb, yp, mb, mp)
        } else {
            (vec![], vec![], 0, 0)
        };

        Ok(ReadingStats {
            books_last_30_days: 0,
            books_all_time,
            pages_last_30_days: 0,
            pages_all_time,
            books_in_progress: 0,
            books_on_shelf: 0,
            books_on_wishlist: 0,
            average_rating,
            books_abandoned,
            average_days_to_finish,
            rating_distribution,
            max_rating_count,
            monthly_books,
            monthly_pages,
            max_monthly_books,
            max_monthly_pages,
            yearly_books,
            yearly_pages,
            max_yearly_books,
            max_yearly_pages,
            pace_distribution,
            format_counts,
        })
    }
}

#[async_trait]
impl StatsRepository for SqlStatsRepository {
    async fn book_summary(&self, user_id: UserId) -> Result<BookSummaryStats, RepositoryError> {
        let uid = user_id.into_inner();

        let total_books: i64 = query_scalar(
            r"SELECT COUNT(*) FROM user_books WHERE user_id = ? AND shelf = 'library'",
        )
        .bind(uid)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;

        let total_authors: i64 = query_scalar(
            r"SELECT COUNT(DISTINCT ba.author_id)
               FROM user_books ub
               JOIN book_authors ba ON ba.book_id = ub.book_id
               WHERE ub.user_id = ? AND ub.shelf = 'library'",
        )
        .bind(uid)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;

        let genre_records: Vec<NameCount> = query_as(
            r"SELECT g.name AS name, COUNT(*) AS count
              FROM user_books ub
              JOIN books b ON b.id = ub.book_id
              JOIN genres g ON g.id IN (b.primary_genre_id, b.secondary_genre_id)
              WHERE ub.user_id = ? AND ub.shelf = 'library'
                AND g.id IS NOT NULL
              GROUP BY g.id
              ORDER BY count DESC",
        )
        .bind(uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let top_author = query_as::<_, NameCount>(
            r"SELECT a.name AS name, COUNT(*) AS count
               FROM user_books ub
               JOIN book_authors ba ON ba.book_id = ub.book_id
               JOIN authors a ON ba.author_id = a.id
               WHERE ub.user_id = ? AND ub.shelf = 'library'
               GROUP BY a.id ORDER BY count DESC LIMIT 1",
        )
        .bind(uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .map(|r| r.name);

        let most_rated_author = self.fetch_most_rated_author(uid, None).await?;
        let most_rated_genre = self.fetch_most_rated_genre(uid, None).await?;

        let (page_distribution, year_distribution) = self.fetch_library_distributions(uid).await?;

        let top_authors_records: Vec<NameCount> = query_as(
            r"SELECT a.name AS name, COUNT(*) AS count
               FROM readings r
               JOIN book_authors ba ON r.book_id = ba.book_id AND ba.role = 'author'
               JOIN authors a ON ba.author_id = a.id
               WHERE r.user_id = ? AND r.status = 'read'
               GROUP BY a.id
               ORDER BY count DESC
               LIMIT 13",
        )
        .bind(uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let (longest, shortest) = self.fetch_library_extremes(uid).await?;

        Ok(build_book_summary(RawBookStats {
            total_books,
            total_authors,
            genre_records,
            top_author,
            most_rated_author,
            most_rated_genre,
            page_distribution,
            year_distribution,
            top_authors_records,
            longest,
            shortest,
        }))
    }

    async fn reading_summary(&self, user_id: UserId) -> Result<ReadingStats, RepositoryError> {
        let uid = user_id.into_inner();

        let mut stats = self.build_reading_stats(uid, None).await?;

        // All-time summary includes current-state counters not relevant to per-year view
        stats.books_last_30_days = query_scalar(
            r"SELECT COUNT(*) FROM readings
               WHERE user_id = ? AND status = 'read' AND finished_at >= date('now', '-30 days')",
        )
        .bind(uid)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)
        .map(|v: i64| v as u64)?;

        stats.pages_last_30_days = query_scalar(
            r"SELECT COALESCE(SUM(bk.page_count), 0) FROM readings r
               JOIN books bk ON r.book_id = bk.id
               WHERE r.user_id = ? AND r.status = 'read' AND r.finished_at >= date('now', '-30 days')
               AND bk.page_count IS NOT NULL",
        )
        .bind(uid)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;

        stats.books_in_progress =
            query_scalar(r"SELECT COUNT(*) FROM readings WHERE user_id = ? AND status = 'reading'")
                .bind(uid)
                .fetch_one(&self.pool)
                .await
                .map_err(db_err)
                .map(|v: i64| v as u64)?;

        stats.books_on_shelf = query_scalar(
            r"SELECT COUNT(*) FROM user_books ub
               WHERE ub.user_id = ? AND ub.shelf = 'library'
               AND NOT EXISTS (
                   SELECT 1 FROM readings r
                   WHERE r.book_id = ub.book_id AND r.user_id = ub.user_id
               )",
        )
        .bind(uid)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)
        .map(|v: i64| v as u64)?;

        stats.books_on_wishlist = query_scalar(
            r"SELECT COUNT(*) FROM user_books WHERE user_id = ? AND shelf = 'wishlist'",
        )
        .bind(uid)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)
        .map(|v: i64| v as u64)?;

        Ok(stats)
    }

    async fn get_cached(&self, user_id: UserId) -> Result<Option<CachedStats>, RepositoryError> {
        let row = sqlx::query(r"SELECT data FROM stats_cache WHERE user_id = ?")
            .bind(user_id.into_inner())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        match row {
            Some(row) => {
                let json: String = row.try_get("data").map_err(db_err)?;
                let stats: CachedStats = serde_json::from_str(&json)
                    .map_err(|err| RepositoryError::unexpected(err.to_string()))?;
                Ok(Some(stats))
            }
            None => Ok(None),
        }
    }

    async fn store_cached(
        &self,
        user_id: UserId,
        stats: &CachedStats,
    ) -> Result<(), RepositoryError> {
        let json = serde_json::to_string(stats)
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        sqlx::query(
            r"INSERT OR REPLACE INTO stats_cache (user_id, data, computed_at)
              VALUES (?, ?, datetime('now'))",
        )
        .bind(user_id.into_inner())
        .bind(&json)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    async fn available_years(&self, user_id: UserId) -> Result<Vec<i32>, RepositoryError> {
        let years: Vec<i32> = query_scalar(
            r"SELECT DISTINCT CAST(strftime('%Y', finished_at) AS INTEGER)
               FROM readings
               WHERE user_id = ? AND status = 'read' AND finished_at IS NOT NULL
               ORDER BY 1 DESC",
        )
        .bind(user_id.into_inner())
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(years)
    }

    async fn book_summary_for_year(
        &self,
        user_id: UserId,
        year: i32,
    ) -> Result<BookSummaryStats, RepositoryError> {
        let uid = user_id.into_inner();

        // All queries for year scope use a CTE that defines the set of books read that year.
        let cte = r"WITH year_books AS (
                   SELECT DISTINCT r.book_id FROM readings r
                   WHERE r.user_id = ? AND r.status = 'read'
                     AND CAST(strftime('%Y', r.finished_at) AS INTEGER) = ?
               )";

        let total_books: i64 = query_scalar(&format!("{cte} SELECT COUNT(*) FROM year_books"))
            .bind(uid)
            .bind(year)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;

        let total_authors: i64 = query_scalar(&format!(
            "{cte} SELECT COUNT(DISTINCT ba.author_id) \
             FROM year_books yb JOIN book_authors ba ON ba.book_id = yb.book_id"
        ))
        .bind(uid)
        .bind(year)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;

        let genre_records: Vec<NameCount> = query_as(&format!(
            "{cte} SELECT g.name AS name, COUNT(*) AS count \
             FROM year_books yb \
             JOIN books b ON b.id = yb.book_id \
             JOIN genres g ON g.id IN (b.primary_genre_id, b.secondary_genre_id) \
             WHERE g.id IS NOT NULL \
             GROUP BY g.id ORDER BY count DESC"
        ))
        .bind(uid)
        .bind(year)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let top_author = query_as::<_, NameCount>(&format!(
            "{cte} SELECT a.name AS name, COUNT(*) AS count \
             FROM year_books yb \
             JOIN book_authors ba ON ba.book_id = yb.book_id \
             JOIN authors a ON ba.author_id = a.id \
             GROUP BY a.id ORDER BY count DESC LIMIT 1"
        ))
        .bind(uid)
        .bind(year)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .map(|r| r.name);

        let most_rated_author = self.fetch_most_rated_author(uid, Some(year)).await?;
        let most_rated_genre = self.fetch_most_rated_genre(uid, Some(year)).await?;

        let (page_distribution, year_distribution) =
            self.fetch_year_distributions(uid, year, cte).await?;

        let top_authors_records: Vec<NameCount> = query_as(
            r"SELECT a.name AS name, COUNT(*) AS count
               FROM readings r
               JOIN book_authors ba ON r.book_id = ba.book_id AND ba.role = 'author'
               JOIN authors a ON ba.author_id = a.id
               WHERE r.user_id = ? AND r.status = 'read'
                 AND CAST(strftime('%Y', r.finished_at) AS INTEGER) = ?
               GROUP BY a.id
               ORDER BY count DESC
               LIMIT 13",
        )
        .bind(uid)
        .bind(year)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let (longest, shortest) = self.fetch_year_extremes(uid, year, cte).await?;

        Ok(build_book_summary(RawBookStats {
            total_books,
            total_authors,
            genre_records,
            top_author,
            most_rated_author,
            most_rated_genre,
            page_distribution,
            year_distribution,
            top_authors_records,
            longest,
            shortest,
        }))
    }

    async fn reading_summary_for_year(
        &self,
        user_id: UserId,
        year: i32,
    ) -> Result<ReadingStats, RepositoryError> {
        self.build_reading_stats(user_id.into_inner(), Some(year))
            .await
    }
}
