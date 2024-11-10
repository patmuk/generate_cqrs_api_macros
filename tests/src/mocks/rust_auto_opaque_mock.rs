#[derive(Clone)]
pub struct RustAutoOpaque<T> {
    pub model: T,
}

impl<T: std::clone::Clone> RustAutoOpaque<T> {
    pub(crate) fn blocking_write(&self) -> T {
        self.clone().model
    }

}
