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

impl<T: Copy + Default> Clone for Wrapper<[T; 64]> {
    fn clone(&self) -> Self {
        let mut result = [T::default(); 64];
        result[..].clone_from_slice(&self.0[..]);
        Wrapper::new(result)
    }
}
