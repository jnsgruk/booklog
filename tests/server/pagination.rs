use booklog::domain::authors::{AuthorSortKey, NewAuthor};
use booklog::domain::listing::{ListRequest, PageSize, SortDirection, SortKey};

use crate::helpers::{create_author_with_name, spawn_app, spawn_app_with_auth};

fn limited(page: u32, size: u32, sort_key: AuthorSortKey) -> ListRequest<AuthorSortKey> {
    ListRequest::new(
        page,
        PageSize::Limited(size),
        sort_key,
        sort_key.default_direction(),
    )
}

#[tokio::test]
async fn listing_with_page_beyond_last_clamps_to_last_page() {
    let app = spawn_app_with_auth().await;
    for i in 0..3 {
        create_author_with_name(&app, &format!("Author {i}")).await;
    }

    // 3 items, page_size=2 → 2 pages. Request page 99.
    let sort_key = AuthorSortKey::default();
    let request = limited(99, 2, sort_key);
    let page = app.author_repo.list(&request, None).await.unwrap();

    // Should clamp to page 2 (last page) and return 1 item
    assert_eq!(page.page, 2);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.total, 3);
}

#[tokio::test]
async fn listing_empty_table_returns_empty_page() {
    let app = spawn_app().await;

    let sort_key = AuthorSortKey::default();
    let request = limited(1, 10, sort_key);
    let page = app.author_repo.list(&request, None).await.unwrap();

    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 0);
    assert_eq!(page.page, 1);
}

#[tokio::test]
async fn listing_with_show_all_returns_every_item() {
    let app = spawn_app_with_auth().await;
    for i in 0..5 {
        create_author_with_name(&app, &format!("Author {i}")).await;
    }

    let sort_key = AuthorSortKey::default();
    let request = ListRequest::show_all(sort_key, sort_key.default_direction());
    let page = app.author_repo.list(&request, None).await.unwrap();

    assert_eq!(page.items.len(), 5);
    assert!(page.showing_all);
    assert_eq!(page.total, 5);
}

#[tokio::test]
async fn listing_with_search_filters_results() {
    let app = spawn_app_with_auth().await;
    create_author_with_name(&app, "Alice Walker").await;
    create_author_with_name(&app, "Bob Dylan").await;
    create_author_with_name(&app, "Alice Munro").await;

    let sort_key = AuthorSortKey::default();
    let request = ListRequest::show_all(sort_key, sort_key.default_direction());
    let page = app.author_repo.list(&request, Some("alice")).await.unwrap();

    assert_eq!(page.items.len(), 2);
    assert!(page.items.iter().all(|a| a.name.contains("Alice")));
}

#[tokio::test]
async fn listing_with_search_no_match_returns_empty() {
    let app = spawn_app_with_auth().await;
    create_author_with_name(&app, "Alice Walker").await;

    let sort_key = AuthorSortKey::default();
    let request = ListRequest::show_all(sort_key, sort_key.default_direction());
    let page = app
        .author_repo
        .list(&request, Some("zzzznotfound"))
        .await
        .unwrap();

    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 0);
}

#[tokio::test]
async fn listing_with_sort_direction_changes_order() {
    let app = spawn_app_with_auth().await;
    // Create with known created_at order
    app.author_repo
        .insert(NewAuthor {
            name: "First".to_string(),
            created_at: Some(chrono::Utc::now() - chrono::Duration::hours(2)),
        })
        .await
        .unwrap();
    app.author_repo
        .insert(NewAuthor {
            name: "Second".to_string(),
            created_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
        })
        .await
        .unwrap();
    app.author_repo
        .insert(NewAuthor {
            name: "Third".to_string(),
            created_at: Some(chrono::Utc::now()),
        })
        .await
        .unwrap();

    let sort_key = AuthorSortKey::CreatedAt;

    // DESC — newest first
    let request = ListRequest::show_all(sort_key, SortDirection::Desc);
    let page = app.author_repo.list(&request, None).await.unwrap();
    assert_eq!(page.items[0].name, "Third");
    assert_eq!(page.items[2].name, "First");

    // ASC — oldest first
    let request = ListRequest::show_all(sort_key, SortDirection::Asc);
    let page = app.author_repo.list(&request, None).await.unwrap();
    assert_eq!(page.items[0].name, "First");
    assert_eq!(page.items[2].name, "Third");
}

#[tokio::test]
async fn listing_page_size_boundaries() {
    let app = spawn_app_with_auth().await;
    for i in 0..4 {
        create_author_with_name(&app, &format!("Author {i}")).await;
    }

    let sort_key = AuthorSortKey::default();

    // Exactly full page
    let request = limited(1, 4, sort_key);
    let page = app.author_repo.list(&request, None).await.unwrap();
    assert_eq!(page.items.len(), 4);
    assert!(!page.has_next());

    // One less than total
    let request = limited(1, 3, sort_key);
    let page = app.author_repo.list(&request, None).await.unwrap();
    assert_eq!(page.items.len(), 3);
    assert!(page.has_next());

    // Page 2 with remainder
    let request = limited(2, 3, sort_key);
    let page = app.author_repo.list(&request, None).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(!page.has_next());
}

#[tokio::test]
async fn listing_with_search_and_pagination() {
    let app = spawn_app_with_auth().await;
    // Create 5 authors matching "test" and 2 that don't
    for i in 0..5 {
        create_author_with_name(&app, &format!("Test Author {i}")).await;
    }
    create_author_with_name(&app, "Bob Dylan").await;
    create_author_with_name(&app, "Carol King").await;

    let sort_key = AuthorSortKey::default();

    // Search "test" with page_size=2 → 5 matching, 3 pages
    let request = limited(1, 2, sort_key);
    let page = app.author_repo.list(&request, Some("test")).await.unwrap();
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.total, 5);
    assert!(page.has_next());

    // Last page of search results
    let request = limited(3, 2, sort_key);
    let page = app.author_repo.list(&request, Some("test")).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(!page.has_next());
}
