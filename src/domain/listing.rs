use std::cmp;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        }
    }

    /// Returns uppercase SQL keyword for ORDER BY clauses.
    pub const fn as_sql(self) -> &'static str {
        match self {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        }
    }
}

pub trait SortKey: Copy + Eq {
    fn default() -> Self;
    fn from_query(value: &str) -> Option<Self>;
    fn query_value(self) -> &'static str;
    fn default_direction(self) -> SortDirection;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PageSize {
    Limited(u32),
    All,
}

impl PageSize {
    pub fn limited(size: u32) -> Self {
        if size == 0 {
            PageSize::All
        } else {
            PageSize::Limited(size)
        }
    }

    pub const fn is_all(self) -> bool {
        matches!(self, PageSize::All)
    }

    pub const fn as_option(self) -> Option<u32> {
        match self {
            PageSize::Limited(value) => Some(value),
            PageSize::All => None,
        }
    }

    pub fn to_query_value(self) -> String {
        match self {
            PageSize::All => "all".to_string(),
            PageSize::Limited(value) => value.to_string(),
        }
    }
}

pub const DEFAULT_PAGE_SIZE: u32 = 10;
pub const MAX_PAGE_SIZE: u32 = 50;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ListRequest<K: SortKey> {
    pub page: u32,
    pub page_size: PageSize,
    pub sort_key: K,
    pub sort_direction: SortDirection,
}

impl<K: SortKey> ListRequest<K> {
    pub fn new(page: u32, page_size: PageSize, sort_key: K, sort_direction: SortDirection) -> Self {
        let page = page.max(1);
        let page_size = match page_size {
            PageSize::Limited(size) => {
                if size == 0 {
                    PageSize::All
                } else {
                    let clamped = cmp::min(size, MAX_PAGE_SIZE).max(1);
                    PageSize::Limited(clamped)
                }
            }
            PageSize::All => PageSize::All,
        };

        Self {
            page,
            page_size,
            sort_key,
            sort_direction,
        }
    }

    pub fn default_query() -> Self {
        let key = K::default();
        Self::new(
            1,
            PageSize::Limited(DEFAULT_PAGE_SIZE),
            key,
            key.default_direction(),
        )
    }

    pub fn show_all(sort_key: K, sort_direction: SortDirection) -> Self {
        Self::new(1, PageSize::All, sort_key, sort_direction)
    }

    pub const fn page(&self) -> u32 {
        self.page
    }

    pub const fn page_size(&self) -> PageSize {
        self.page_size
    }

    pub const fn sort_key(&self) -> K {
        self.sort_key
    }

    pub const fn sort_direction(&self) -> SortDirection {
        self.sort_direction
    }

    pub fn with_page(self, page: u32) -> Self {
        Self {
            page: page.max(1),
            ..self
        }
    }

    pub fn with_page_size(self, page_size: PageSize) -> Self {
        Self::new(self.page, page_size, self.sort_key, self.sort_direction)
    }

    pub fn with_sort(self, key: K) -> Self {
        let direction = if key == self.sort_key {
            self.sort_direction.opposite()
        } else {
            key.default_direction()
        };
        Self::new(self.page, self.page_size, key, direction)
    }

    pub fn with_sort_and_direction(self, key: K, direction: SortDirection) -> Self {
        Self::new(self.page, self.page_size, key, direction)
    }

    pub fn ensure_page_within(self, total: u64) -> Self {
        if matches!(self.page_size, PageSize::All) {
            return Self::new(1, PageSize::All, self.sort_key, self.sort_direction);
        }

        let Some(limit) = self.page_size.as_option() else {
            return self;
        };

        if total == 0 {
            return Self::new(1, self.page_size, self.sort_key, self.sort_direction);
        }

        let last_page = (total.div_ceil(u64::from(limit))) as u32;
        let adjusted_page = self.page.min(last_page.max(1));
        Self::new(
            adjusted_page,
            self.page_size,
            self.sort_key,
            self.sort_direction,
        )
    }
}

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub showing_all: bool,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, page: u32, page_size: u32, total: u64, showing_all: bool) -> Self {
        Self {
            items,
            page: page.max(1),
            page_size: page_size.max(1),
            total,
            showing_all,
        }
    }

    pub fn total_pages(&self) -> u32 {
        if self.total == 0 || self.showing_all {
            1
        } else {
            let size = u64::from(self.page_size);
            (self.total.div_ceil(size)) as u32
        }
    }

    pub fn has_previous(&self) -> bool {
        !self.showing_all && self.page > 1
    }

    pub fn has_next(&self) -> bool {
        !self.showing_all && self.page < self.total_pages()
    }

    pub fn start_index(&self) -> u64 {
        if self.total == 0 {
            0
        } else {
            u64::from(self.page - 1) * u64::from(self.page_size) + 1
        }
    }

    pub fn end_index(&self) -> u64 {
        if self.total == 0 {
            0
        } else {
            self.start_index() + self.items.len() as u64 - 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal SortKey for testing
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    enum TestKey {
        A,
        B,
    }

    impl SortKey for TestKey {
        fn default() -> Self {
            TestKey::A
        }
        fn from_query(value: &str) -> Option<Self> {
            match value {
                "a" => Some(TestKey::A),
                "b" => Some(TestKey::B),
                _ => None,
            }
        }
        fn query_value(self) -> &'static str {
            match self {
                TestKey::A => "a",
                TestKey::B => "b",
            }
        }
        fn default_direction(self) -> SortDirection {
            match self {
                TestKey::A => SortDirection::Desc,
                TestKey::B => SortDirection::Asc,
            }
        }
    }

    // --- SortDirection ---

    #[test]
    fn sort_direction_opposite() {
        assert_eq!(SortDirection::Asc.opposite(), SortDirection::Desc);
        assert_eq!(SortDirection::Desc.opposite(), SortDirection::Asc);
    }

    #[test]
    fn sort_direction_as_str() {
        assert_eq!(SortDirection::Asc.as_str(), "asc");
        assert_eq!(SortDirection::Desc.as_str(), "desc");
    }

    // --- PageSize ---

    #[test]
    fn page_size_zero_becomes_all() {
        assert_eq!(PageSize::limited(0), PageSize::All);
    }

    #[test]
    fn page_size_nonzero() {
        assert_eq!(PageSize::limited(10), PageSize::Limited(10));
    }

    #[test]
    fn page_size_is_all() {
        assert!(PageSize::All.is_all());
        assert!(!PageSize::Limited(10).is_all());
    }

    #[test]
    fn page_size_as_option() {
        assert_eq!(PageSize::All.as_option(), None);
        assert_eq!(PageSize::Limited(5).as_option(), Some(5));
    }

    #[test]
    fn page_size_to_query_value() {
        assert_eq!(PageSize::All.to_query_value(), "all");
        assert_eq!(PageSize::Limited(25).to_query_value(), "25");
    }

    // --- ListRequest ---

    #[test]
    fn list_request_clamps_page_to_minimum_1() {
        let req = ListRequest::new(0, PageSize::Limited(10), TestKey::A, SortDirection::Desc);
        assert_eq!(req.page(), 1);
    }

    #[test]
    fn list_request_clamps_page_size_to_max() {
        let req = ListRequest::new(1, PageSize::Limited(100), TestKey::A, SortDirection::Desc);
        assert_eq!(req.page_size(), PageSize::Limited(MAX_PAGE_SIZE));
    }

    #[test]
    fn list_request_zero_page_size_becomes_all() {
        let req = ListRequest::new(1, PageSize::Limited(0), TestKey::A, SortDirection::Desc);
        assert_eq!(req.page_size(), PageSize::All);
    }

    #[test]
    fn list_request_preserves_all() {
        let req = ListRequest::new(1, PageSize::All, TestKey::A, SortDirection::Desc);
        assert_eq!(req.page_size(), PageSize::All);
    }

    #[test]
    fn list_request_default_query() {
        let req = ListRequest::<TestKey>::default_query();
        assert_eq!(req.page(), 1);
        assert_eq!(req.page_size(), PageSize::Limited(DEFAULT_PAGE_SIZE));
        assert_eq!(req.sort_key(), TestKey::A);
        assert_eq!(req.sort_direction(), SortDirection::Desc);
    }

    #[test]
    fn with_sort_toggles_direction_for_same_key() {
        let req = ListRequest::new(1, PageSize::Limited(10), TestKey::A, SortDirection::Desc);
        let toggled = req.with_sort(TestKey::A);
        assert_eq!(toggled.sort_key(), TestKey::A);
        assert_eq!(toggled.sort_direction(), SortDirection::Asc);
    }

    #[test]
    fn with_sort_uses_default_direction_for_different_key() {
        let req = ListRequest::new(1, PageSize::Limited(10), TestKey::A, SortDirection::Desc);
        let changed = req.with_sort(TestKey::B);
        assert_eq!(changed.sort_key(), TestKey::B);
        assert_eq!(changed.sort_direction(), SortDirection::Asc); // B's default
    }

    #[test]
    fn ensure_page_within_empty_total() {
        let req = ListRequest::new(5, PageSize::Limited(10), TestKey::A, SortDirection::Desc);
        let adjusted = req.ensure_page_within(0);
        assert_eq!(adjusted.page(), 1);
    }

    #[test]
    fn ensure_page_within_clamps_past_last_page() {
        let req = ListRequest::new(10, PageSize::Limited(10), TestKey::A, SortDirection::Desc);
        let adjusted = req.ensure_page_within(25); // 3 pages
        assert_eq!(adjusted.page(), 3);
    }

    #[test]
    fn ensure_page_within_keeps_valid_page() {
        let req = ListRequest::new(2, PageSize::Limited(10), TestKey::A, SortDirection::Desc);
        let adjusted = req.ensure_page_within(25);
        assert_eq!(adjusted.page(), 2);
    }

    #[test]
    fn ensure_page_within_show_all_resets_to_page_1() {
        let req = ListRequest::new(5, PageSize::All, TestKey::A, SortDirection::Desc);
        let adjusted = req.ensure_page_within(100);
        assert_eq!(adjusted.page(), 1);
    }

    // --- Page ---

    #[test]
    fn page_total_pages() {
        let page: Page<()> = Page::new(vec![], 1, 10, 25, false);
        assert_eq!(page.total_pages(), 3);
    }

    #[test]
    fn page_total_pages_empty() {
        let page: Page<()> = Page::new(vec![], 1, 10, 0, false);
        assert_eq!(page.total_pages(), 1);
    }

    #[test]
    fn page_total_pages_showing_all() {
        let page: Page<()> = Page::new(vec![], 1, 10, 100, true);
        assert_eq!(page.total_pages(), 1);
    }

    #[test]
    fn page_has_previous() {
        let page1: Page<()> = Page::new(vec![], 1, 10, 25, false);
        assert!(!page1.has_previous());

        let page2: Page<()> = Page::new(vec![], 2, 10, 25, false);
        assert!(page2.has_previous());
    }

    #[test]
    fn page_has_next() {
        let last: Page<()> = Page::new(vec![], 3, 10, 25, false);
        assert!(!last.has_next());

        let first: Page<()> = Page::new(vec![], 1, 10, 25, false);
        assert!(first.has_next());
    }

    #[test]
    fn page_has_no_navigation_when_showing_all() {
        let page: Page<()> = Page::new(vec![], 2, 10, 100, true);
        assert!(!page.has_previous());
        assert!(!page.has_next());
    }

    #[test]
    fn page_start_end_index() {
        let page: Page<i32> = Page::new(vec![1, 2, 3], 2, 10, 25, false);
        assert_eq!(page.start_index(), 11);
        assert_eq!(page.end_index(), 13);
    }

    #[test]
    fn page_start_end_index_empty() {
        let page: Page<i32> = Page::new(vec![], 1, 10, 0, false);
        assert_eq!(page.start_index(), 0);
        assert_eq!(page.end_index(), 0);
    }
}
