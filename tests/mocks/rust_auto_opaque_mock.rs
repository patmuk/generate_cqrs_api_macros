#[derive(Default, Clone, Debug)]
pub(crate) struct RustAutoOpaque<M> {
    pub model: M,
}

impl<M: CqrsModel> RustAutoOpaque<M> {
    fn new(model: M) -> Self {
        RustAutoOpaque { model }
    }
}

impl<T: std::clone::Clone> RustAutoOpaque<T> {
    pub(crate) fn blocking_write(&self) -> T {
        self.clone().model
    }
    pub(crate) fn blocking_read(&self) -> T {
        self.clone().model
    }
}
