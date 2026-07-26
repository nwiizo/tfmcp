const DEFAULT_PAGE_SIZE: u16 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageParams {
    pub number: u16,
    pub size: u16,
}

impl Default for PageParams {
    fn default() -> Self {
        Self {
            number: 1,
            size: DEFAULT_PAGE_SIZE,
        }
    }
}

impl PageParams {
    pub fn new(number: Option<u16>, size: Option<u16>) -> Self {
        Self {
            number: number.unwrap_or(1).max(1),
            size: size.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, 100),
        }
    }

    pub(crate) fn query(&self) -> String {
        format!("?page[number]={}&page[size]={}", self.number, self.size)
    }
}
