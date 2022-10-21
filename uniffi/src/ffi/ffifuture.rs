use core::future::Future;
use core::mem::ManuallyDrop;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::sync::Arc;
use core::ffi::c_void;


/// Converts a closure into a [`Waker`].
///
/// The closure gets called every time the waker is woken.
pub fn waker_fn<F: Fn() + Send + Sync + 'static>(f: F) -> Waker {
    let raw = Arc::into_raw(Arc::new(f)) as *const ();
    let vtable = &Helper::<F>::VTABLE;
    unsafe { Waker::from_raw(RawWaker::new(raw, vtable)) }
}

struct Helper<F>(F);

impl<F: Fn() + Send + Sync + 'static> Helper<F> {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_waker,
        Self::wake,
        Self::wake_by_ref,
        Self::drop_waker,
    );

    unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
        let arc = ManuallyDrop::new(Arc::from_raw(ptr as *const F));
        core::mem::forget(arc.clone());
        RawWaker::new(ptr, &Self::VTABLE)
    }

    unsafe fn wake(ptr: *const ()) {
        let arc = Arc::from_raw(ptr as *const F);
        (arc)();
    }

    unsafe fn wake_by_ref(ptr: *const ()) {
        let arc = ManuallyDrop::new(Arc::from_raw(ptr as *const F));
        (arc)();
    }

    unsafe fn drop_waker(ptr: *const ()) {
        drop(Arc::from_raw(ptr as *const F));
    }
}

fn ffi_waker(_post_cobject: isize, port: i64) -> Waker {
    waker_fn(move || unsafe {
        let post_cobject: extern "C" fn(i64, *const c_void) =
            core::mem::transmute(_post_cobject);
        let obj: i32 = 0;
        post_cobject(port, &obj as *const _ as *const _);
    })
}


#[repr(transparent)]
pub struct FfiFuture<T: Send + 'static>(Pin<Box<dyn Future<Output = T> + Send + 'static>>);

impl<T: Send + 'static> FfiFuture<T> {
    pub fn new(f: impl Future<Output = T> + Send + 'static) -> Self {
        Self(Box::pin(f))
    }

    pub fn poll(&mut self, post_cobject: isize, port: i64) -> Option<T> {
        let waker = ffi_waker(post_cobject, port);
        let mut ctx = Context::from_waker(&waker);
        match Pin::new(&mut self.0).poll(&mut ctx) {
            Poll::Ready(res) => Some(res),
            Poll::Pending => None,
        }
    }
}

impl<T: Send + 'static + Default> super::FfiDefault for FfiFuture<T> {
    fn ffi_default() -> Self {
        FfiFuture::new(
            core::future::ready(T::default())
        )
    }
}