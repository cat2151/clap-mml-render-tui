//! 先読み（background）ジョブを処理している間だけ preparation スレッドの優先度を下げる。
//!
//! rubberband のオフラインストレッチは 1 コアを食い切るため、通常優先度のまま裏で走らせると
//! 同時に鳴っているオーディオ出力スレッドを押しのけて音が途切れうる。
//! 前景（ユーザー操作による準備）は無音で待たせている最中なので、優先度は下げない。

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL, THREAD_PRIORITY_NORMAL,
    };

    pub struct ThreadPriorityGuard;

    impl ThreadPriorityGuard {
        pub fn below_normal() -> Self {
            unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) };
            Self
        }
    }

    impl Drop for ThreadPriorityGuard {
        fn drop(&mut self) {
            unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_NORMAL) };
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct ThreadPriorityGuard;

    impl ThreadPriorityGuard {
        pub fn below_normal() -> Self {
            Self
        }
    }
}

pub use imp::ThreadPriorityGuard;
