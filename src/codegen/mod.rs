mod buffer;
pub(crate) mod js;

#[derive(Default, Clone)]
pub(crate) struct GenerateOpts {
    pub(crate) debug: bool,
    pub(crate) vacation_from_address: Option<String>,
}
