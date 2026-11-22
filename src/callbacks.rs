//! UI 线程上的回调登记表。
//!
//! C ABI 从不释放回调的 `user_data`：事件监听、定时器、尺寸变化与脚本错误回调都只在 UI 线程上调用，
//! 所以闭包放在线程局部的表里，`user_data` 只是表里的编号。执行时先把闭包取出来（槽位标成「执行中」），
//! 执行完再放回；执行期间被注销（例如监听器移除自己）就不放回，直接释放。闭包的 drop 一律发生在
//! 释放表的借用之后——闭包可能拥有 `Window` 之类的对象，它们析构时会再回到这张表注销自己。
//!
//! 线程退出时表随线程局部存储销毁，表里的闭包在销毁过程中被 drop，它们拥有的对象同样会回到这张表
//! （例如监听器闭包捕获了 `Document` 的克隆）。所以注销 / 释放路径一律用 `try_with`：表已在销毁时
//! 什么都不做（剩下的闭包随表一起 drop），不能 panic——线程局部析构里的 panic 会直接中止进程。

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// 平台定时器与 idle 回调的所有者编号（元素 / 窗口 / 引擎用各自的指针地址，不会是 1）。
pub(crate) const TIMER_OWNER: usize = 1;

enum Slot {
    Ready(Box<dyn Any>),
    Running,
    RunningRemoved,
}

struct Entry {
    slot: Slot,
    once: bool,
    owner: usize,
    group: String,
}

thread_local! {
    static REGISTRY: RefCell<HashMap<usize, Entry>> = RefCell::new(HashMap::new());
    static NEXT_TOKEN: Cell<usize> = const { Cell::new(1) };
}

/// 登记一个回调，返回作为 `user_data` 传给 C ABI 的编号。
pub(crate) fn register<T: 'static>(value: T, once: bool, owner: usize, group: &str) -> usize {
    let token = NEXT_TOKEN.with(|next| {
        let token = next.get();
        next.set(token + 1);
        token
    });
    REGISTRY.with(|registry| {
        registry.borrow_mut().insert(
            token,
            Entry {
                slot: Slot::Ready(Box::new(value)),
                once,
                owner,
                group: group.to_owned(),
            },
        );
    });
    token
}

/// 登记之后才知道分组（例如定时器 id）时改分组。
pub(crate) fn regroup(token: usize, group: &str) {
    REGISTRY.with(|registry| {
        if let Some(entry) = registry.borrow_mut().get_mut(&token) {
            entry.group = group.to_owned();
        }
    });
}

fn detach_where(predicate: impl Fn(usize, &Entry) -> bool) {
    let removed = REGISTRY.try_with(|registry| {
        let mut registry = registry.borrow_mut();
        let tokens: Vec<usize> = registry
            .iter()
            .filter(|(token, entry)| predicate(**token, entry))
            .map(|(token, _)| *token)
            .collect();
        let mut removed = Vec::new();
        for token in tokens {
            let ready = matches!(
                registry.get(&token).map(|entry| &entry.slot),
                Some(Slot::Ready(_))
            );
            if ready {
                if let Some(Entry {
                    slot: Slot::Ready(value),
                    ..
                }) = registry.remove(&token)
                {
                    removed.push(value);
                }
            } else if let Some(entry) = registry.get_mut(&token) {
                if matches!(entry.slot, Slot::Running) {
                    entry.slot = Slot::RunningRemoved;
                }
            }
        }
        removed
    });
    // 表已在销毁（线程退出）：剩下的闭包随表一起 drop，这里没有可注销的。
    drop(removed.unwrap_or_default());
}

/// 注销一个回调（正在执行时等它返回后再释放）。
pub(crate) fn unregister(token: usize) {
    detach_where(|candidate, _| candidate == token);
}

/// 注销某个所有者某一分组下的全部回调。
pub(crate) fn unregister_group(owner: usize, group: &str) {
    detach_where(|_, entry| entry.owner == owner && entry.group == group);
}

/// 注销某个所有者的全部回调。
pub(crate) fn unregister_owner(owner: usize) {
    detach_where(|_, entry| entry.owner == owner);
}

/// 注销某个所有者分组名以 `prefix` 开头的全部回调（例如全部快捷键）。
pub(crate) fn unregister_where_group_starts_with(owner: usize, prefix: &str) {
    detach_where(|_, entry| entry.owner == owner && entry.group.starts_with(prefix));
}

thread_local! {
    static RELEASED: RefCell<Vec<Box<dyn Any>>> = const { RefCell::new(Vec::new()) };
}

/// C ABI 的 `release_fn` 通知不再引用 `token` 时调用。`release_fn` 可能在文档析构途中被调用，
/// 那时不能回到 KUI，而闭包 drop 时可能释放它拥有的元素等对象，所以这里只把闭包移进待释放列表，
/// 由 [`drain_released`] 在安全点 drop。正在执行的回调照常等它返回后由 `invoke` 释放。
pub(crate) fn release_deferred(token: usize) {
    let removed = REGISTRY.try_with(|registry| {
        let mut registry = registry.borrow_mut();
        let running = match registry.get(&token) {
            Some(entry) => !matches!(entry.slot, Slot::Ready(_)),
            None => return None,
        };
        if running {
            registry.get_mut(&token).expect("checked").slot = Slot::RunningRemoved;
            return None;
        }
        match registry.remove(&token).map(|entry| entry.slot) {
            Some(Slot::Ready(value)) => Some(value),
            _ => None,
        }
    });
    if let Ok(Some(value)) = removed {
        // 待释放列表也已在销毁时，闭包（连同 value）随 try_with 的失败就地 drop：线程正在退出。
        let _ = RELEASED.try_with(|released| released.borrow_mut().push(value));
    }
}

/// drop [`release_deferred`] 攒下的闭包。只在不处于 KUI 调用内部的安全点调用。
pub(crate) fn drain_released() {
    let items = RELEASED
        .try_with(|released| std::mem::take(&mut *released.borrow_mut()))
        .unwrap_or_default();
    drop(items);
}

/// 取出编号对应的回调并执行；回调里的 panic 被拦下，不会穿过 C ABI。
pub(crate) fn invoke<T: 'static>(token: usize, call: impl FnOnce(&mut T)) {
    let taken = REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let entry = registry.get_mut(&token)?;
        match std::mem::replace(&mut entry.slot, Slot::Running) {
            Slot::Ready(value) => Some(value),
            other => {
                entry.slot = other;
                None
            }
        }
    });
    let Some(mut value) = taken else { return };
    if let Some(callback) = value.downcast_mut::<T>() {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(callback)));
    }
    let released = REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let keep = match registry.get(&token) {
            Some(entry) => matches!(entry.slot, Slot::Running) && !entry.once,
            None => false,
        };
        if keep {
            registry.get_mut(&token).expect("checked").slot = Slot::Ready(value);
            None
        } else {
            registry.remove(&token);
            Some(value)
        }
    });
    drop(released);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    type Counter = Box<dyn FnMut()>;

    #[test]
    fn once_entries_are_released_after_running() {
        let hits = Rc::new(Cell::new(0));
        let h = Rc::clone(&hits);
        let token = register::<Counter>(Box::new(move || h.set(h.get() + 1)), true, 10, "g");
        invoke::<Counter>(token, |f| f());
        invoke::<Counter>(token, |f| f());
        assert_eq!(hits.get(), 1);
        assert_eq!(Rc::strong_count(&hits), 1, "once 回调执行后已释放");
    }

    #[test]
    fn unregister_during_own_call_releases_after_return() {
        let token_cell = Rc::new(Cell::new(0usize));
        let t = Rc::clone(&token_cell);
        let hits = Rc::new(Cell::new(0));
        let h = Rc::clone(&hits);
        let token = register::<Counter>(
            Box::new(move || {
                h.set(h.get() + 1);
                unregister(t.get());
            }),
            false,
            11,
            "g",
        );
        token_cell.set(token);
        invoke::<Counter>(token, |f| f());
        invoke::<Counter>(token, |f| f());
        assert_eq!(hits.get(), 1);
        assert_eq!(Rc::strong_count(&hits), 1);
    }

    struct UnregisterOnDrop(usize);
    impl Drop for UnregisterOnDrop {
        fn drop(&mut self) {
            unregister(self.0);
        }
    }

    #[test]
    fn dropping_a_closure_may_reenter_the_registry() {
        let other = register::<Counter>(Box::new(|| {}), false, 12, "g");
        let guard = UnregisterOnDrop(other);
        let token = register::<Counter>(
            Box::new(move || {
                let _keep = &guard;
            }),
            false,
            12,
            "h",
        );
        unregister_group(12, "h");
        let _ = token;
        assert!(REGISTRY.with(|r| !r.borrow().contains_key(&other)));
    }

    /// 线程退出时登记表随线程局部存储销毁，表里的闭包在销毁过程中被 drop；闭包拥有的对象（例如捕获的
    /// `Document` 克隆：文档 → 监听器 → 闭包 → 文档 的循环）析构时会回到这张表注销 / 释放。以前这里
    /// 访问已在销毁的线程局部存储而 panic，进程以「thread local panicked on drop」中止（2026-11-18）。
    #[test]
    fn reentering_the_registry_during_thread_teardown_does_not_panic() {
        struct ReleaseOnDrop(usize);
        impl Drop for ReleaseOnDrop {
            fn drop(&mut self) {
                release_deferred(self.0);
                unregister(self.0);
                unregister_owner(30);
                drain_released();
            }
        }
        let worker = std::thread::spawn(|| {
            let other = register::<Counter>(Box::new(|| {}), false, 30, "g");
            register::<ReleaseOnDrop>(ReleaseOnDrop(other), false, 30, "g");
        });
        assert!(worker.join().is_ok());
    }

    #[test]
    fn panics_are_contained_and_the_entry_survives() {
        let hits = Rc::new(Cell::new(0));
        let h = Rc::clone(&hits);
        let token = register::<Counter>(
            Box::new(move || {
                h.set(h.get() + 1);
                panic!("probe");
            }),
            false,
            13,
            "g",
        );
        invoke::<Counter>(token, |f| f());
        invoke::<Counter>(token, |f| f());
        assert_eq!(hits.get(), 2);
        unregister_owner(13);
        assert_eq!(Rc::strong_count(&hits), 1);
    }
}
