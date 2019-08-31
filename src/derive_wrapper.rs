pub struct Wrapper<T>(pub T);

impl<T> Wrapper<T> {
    pub fn new(t: T) -> Wrapper<T> {
        Wrapper(t)
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Wrapper<[T; 64]> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0[..].fmt(f)
    }
}

impl<T: PartialEq> PartialEq for Wrapper<[T; 64]> {
    fn eq(&self, other: &Wrapper<[T; 64]>) -> bool {
        self.0[..] == other.0[..]
    }
}
