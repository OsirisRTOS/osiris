use core::borrow::Borrow;

pub trait Get<Idx: ?Sized> {
    type Output: ?Sized;

    fn get<K: Borrow<Idx>>(&self, index: K) -> Option<&Self::Output>;
}

pub trait GetMut<Idx: ?Sized>: Get<Idx> {
    fn get_mut<K: Borrow<Idx>>(&mut self, index: K) -> Option<&mut Self::Output>;

    // Getting multiple disjoint mutable references at once
    fn get2_mut<K: Borrow<Idx>>(
        &mut self,
        index1: K,
        index2: K,
    ) -> (Option<&mut Self::Output>, Option<&mut Self::Output>);
    fn get3_mut<K: Borrow<Idx>>(
        &mut self,
        index1: K,
        index2: K,
        index3: K,
    ) -> (
        Option<&mut Self::Output>,
        Option<&mut Self::Output>,
        Option<&mut Self::Output>,
    );
}

pub trait ToIndex {
    fn to_index<Q: Borrow<Self>>(index: Option<Q>) -> usize;
}

impl ToIndex for usize {
    fn to_index<Q: Borrow<Self>>(index: Option<Q>) -> usize {
        index.map_or(0, |i| *i.borrow())
    }
}

pub trait Project<P> {
    fn project(&self) -> Option<&P>;
    fn project_mut(&mut self) -> Option<&mut P>;
}
