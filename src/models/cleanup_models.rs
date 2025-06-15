
#[derive(Debug, Default)]

pub struct ActivityType {
    pub container: Option<String>,
    pub image: Option<String>,
    pub all_tars: Option<String>,
    pub tar: Option<String>,
    pub ports: Option<Vec<i32>>,
}

impl ActivityType {
    pub fn new(
        container: Option<String>,
        image: Option<String>,
        all_tars: Option<String>,
        tar: Option<String>,
        ports: Option<Vec<i32>>,
    ) -> Self {
        ActivityType {
            container,
            image,
            all_tars,
            tar,
            ports,
        }
    }
}
pub struct CleanupService;
