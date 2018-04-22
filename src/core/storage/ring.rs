use {
    Error,
    Result,
};

/// Ring/bounded buffer of T's.
#[derive(Clone, Debug)]
pub struct Ring<T> {
    buffer: Vec<T>,
    begin: usize,
    len: usize,
}

impl<T> From<Vec<T>> for Ring<T> {
    fn from(buffer: Vec<T>) -> Ring<T> {
        Ring {
            buffer,
            begin: 0,
            len: 0,
        }
    }
}

impl<T> Ring<T> {
    /// Applies f on the head of the buffer or returns an error if the buffer
    /// is empty. Dequeue's the element f was applied on.
    ///
    /// # Returns
    ///
    /// An error or the result of f.
    pub fn dequeue_with<'a, F, R>(&'a mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'a mut T) -> R,
    {
        self.dequeue_maybe(|x| Ok(f(x)))
    }

    /// Similar to dequeue_with(...) except cancels the dequeue operation if
    /// f returns an error.
    ///
    /// # Returns
    ///
    /// An error or the result of f.
    pub fn dequeue_maybe<'a, F, R>(&'a mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'a mut T) -> Result<R>,
    {
        if self.len == 0 {
            return Err(Error::Exhausted);
        }

        let buffer_len = self.buffer.len();

        match f(&mut self.buffer[self.begin]) {
            Err(err) => Err(err),
            Ok(res) => {
                self.begin = (self.begin + 1) % buffer_len;
                self.len -= 1;
                Ok(res)
            }
        }
    }

    /// Applies f on the head of the buffer (so that f can mutate the T as
    /// desired) or returns an error if the buffer is full. Enqueue's the
    /// element f was applied on.
    ///
    /// # Returns
    ///
    /// An error or the result of f.
    pub fn enqueue_with<'a, F, R>(&'a mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'a mut T) -> R,
    {
        self.enqueue_maybe(|x| Ok(f(x)))
    }

    /// Similar to dequeue_with(...) except cancels the enqueue operation if
    /// f returns an error.
    ///
    /// # Returns
    ///
    /// An error or the result of f.
    pub fn enqueue_maybe<'a, F, R>(&'a mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'a mut T) -> Result<R>,
    {
        if self.len == self.buffer.len() {
            return Err(Error::Exhausted);
        }

        let idx = (self.begin + self.len) % self.buffer.len();

        match f(&mut self.buffer[idx]) {
            Err(err) => Err(err),
            Ok(res) => {
                self.len += 1;
                Ok(res)
            }
        }
    }

    /// Returns the current number of items in the ring.
    pub fn len(&self) -> usize {
        self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dequeue_when_empty() {
        let mut ring = Ring::from(vec![0; 1]);
        assert_eq!(ring.len(), 0);
        assert_matches!(ring.dequeue_with(|_| {}), Err(Error::Exhausted));
    }

    #[test]
    fn test_dequeue_maybe_with_error() {
        let mut ring = Ring::from(vec![0; 1]);
        assert_eq!(ring.len(), 0);
        ring.enqueue_with(|i| *i = 1).unwrap();
        assert_eq!(ring.len(), 1);
        assert_matches!(
            ring.dequeue_maybe(|i| if *i == 1 { Err(Error::Ignored) } else { Ok(()) }),
            Err(Error::Ignored)
        );
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn test_enqueue_when_full() {
        let mut ring = Ring::from(vec![0; 1]);
        assert_eq!(ring.len(), 0);
        assert_matches!(ring.enqueue_with(|_| {}), Ok(()));
        assert_eq!(ring.len(), 1);
        assert_matches!(ring.enqueue_with(|_| {}), Err(Error::Exhausted));
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn test_enqueue_maybe_with_error() {
        let mut ring = Ring::from(vec![0; 1]);
        assert_eq!(ring.len(), 0);
        assert_matches!(
            ring.enqueue_maybe(|i| if *i == 0 { Err(Error::Ignored) } else { Ok(()) }),
            Err(Error::Ignored)
        );
        assert_eq!(ring.len(), 0);
        assert_matches!(ring.dequeue_with(|_| {}), Err(Error::Exhausted));
    }

    #[test]
    fn test_enqueue_and_dequeue() {
        let mut ring = Ring::from(vec![0; 4]);
        assert_matches!(ring.enqueue_with(|i| *i = 1), Ok(()));
        assert_matches!(ring.enqueue_with(|i| *i = 2), Ok(()));
        assert_eq!(ring.dequeue_with(|i| *i).unwrap(), 1);
        assert_matches!(ring.enqueue_with(|i| *i = 3), Ok(()));
        assert_eq!(ring.dequeue_with(|i| *i).unwrap(), 2);
        assert_eq!(ring.dequeue_with(|i| *i).unwrap(), 3);
        assert_matches!(ring.dequeue_with(|_| {}), Err(Error::Exhausted));
    }
}
