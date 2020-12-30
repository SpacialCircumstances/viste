use crate::{RValue, RefWrapper};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

pub trait RValueExt<T>: RValue<T> {
    fn select<U, M: Fn(&T) -> &U + Clone>(&self, mapper: M) -> SelectRValue<T, U, M, Self> {
        SelectRValue(self.clone(), mapper, PhantomData::default())
    }
}

impl<T, R: RValue<T>> RValueExt<T> for R {}

pub struct OwnedRValue<T>(Rc<RefCell<T>>);

impl<T> OwnedRValue<T> {
    pub fn new(c: Rc<RefCell<T>>) -> Self {
        Self(c)
    }
}

impl<T> Clone for OwnedRValue<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> RValue<T> for OwnedRValue<T> {
    fn data(&self) -> RefWrapper<T> {
        RefWrapper::Ref(self.0.borrow())
    }
}

pub struct SelectRValue<T, T2, M: Fn(&T) -> &T2 + Clone, R: RValue<T>>(R, M, PhantomData<T>);

impl<T, T2, M: Fn(&T) -> &T2 + Clone, R: RValue<T>> Clone for SelectRValue<T, T2, M, R> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData::default())
    }
}

impl<T, T2, M: Fn(&T) -> &T2 + Clone, R: RValue<T>> RValue<T2> for SelectRValue<T, T2, M, R> {
    fn data(&self) -> RefWrapper<T2> {
        RefWrapper::map(self.0.data(), &self.1)
    }
}

pub struct ConstantRValue<T>(Rc<T>);

impl<T> ConstantRValue<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(value))
    }
}

impl<T> Clone for ConstantRValue<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> RValue<T> for ConstantRValue<T> {
    fn data(&self) -> RefWrapper<T> {
        RefWrapper::Direct(std::borrow::Borrow::borrow(&self.0))
    }
}
