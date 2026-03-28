use core::borrow::Borrow;
use core::marker::PhantomData;

use super::traits::{Get, GetMut, Project, ToIndex};

pub struct ViewMut<'a, K: ?Sized + ToIndex, P, S: GetMut<K>>
where
    S::Output: Project<P>,
{
    data: &'a mut S,
    _k: PhantomData<K>,
    _proj: PhantomData<P>,
}
   
impl<'a, K: ?Sized + ToIndex, P, S: GetMut<K>> ViewMut<'a, K, P, S>
where
    S::Output: Project<P>,
{
    pub fn new(data: &'a mut S) -> Self {
        Self {
            data,
            _k: PhantomData,
            _proj: PhantomData,
        }
    }

    pub fn with<F: FnOnce(&mut Self) -> R, R>(data: &'a mut S, f: F) -> R {
        let mut view = Self::new(data);
        f(&mut view)
    }
}

impl<'a, K: ?Sized + ToIndex, P, S: GetMut<K>> Get<K> for ViewMut<'a, K, P, S>
where
    S::Output: Project<P>,
{
    type Output = P;

    fn get<Q: Borrow<K>>(&self, idx: Q) -> Option<&P> {
        self.data.get(idx).and_then(Project::project)
    }
}

impl<'a, K: ?Sized + ToIndex, P, S: GetMut<K>> GetMut<K> for ViewMut<'a, K, P, S>
where
    S::Output: Project<P>,
{
    fn get_mut<Q: Borrow<K>>(&mut self, idx: Q) -> Option<&mut P> {
        self.data.get_mut(idx).and_then(Project::project_mut)
    }

    fn get2_mut<Q: Borrow<K>>(&mut self, idx1: Q, idx2: Q) -> (Option<&mut P>, Option<&mut P>) {
        let (a, b) = self.data.get2_mut(idx1, idx2);
        (
            a.and_then(Project::project_mut),
            b.and_then(Project::project_mut),
        )
    }

    fn get3_mut<Q: Borrow<K>>(&mut self, idx1: Q, idx2: Q, idx3: Q) -> (Option<&mut P>, Option<&mut P>, Option<&mut P>) {
        let (a, b, c) = self.data.get3_mut(idx1, idx2, idx3);
        (
            a.and_then(Project::project_mut),
            b.and_then(Project::project_mut),
            c.and_then(Project::project_mut),
        )
    }
}