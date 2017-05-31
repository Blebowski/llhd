// Copyright (c) 2017 Fabian Schuiki

use std;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use ty::Type;
use konst::Const;


pub trait Value {
	/// Get the unique ID of the value.
	fn id(&self) -> ValueId;
	/// Get the type of the value.
	fn ty(&self) -> Type;
	/// Get the optional name of the value.
	fn name(&self) -> Option<&str> { None }
	/// Whether this value is global or not. Global values are considered during
	/// linking, and are visible in a module's symbol table. Local values are
	/// not, and are only visible within the surrounding context (module or
	/// unit).
	fn is_global(&self) -> bool { false }
}


/// A reference to a value in a module.
#[derive(Debug, Clone)]
pub enum ValueRef {
	Inst(InstRef),
	Block(BlockRef),
	Argument(ArgumentRef),
	Function(FunctionRef),
	Process(ProcessRef),
	Entity(EntityRef),
	Global,
	Const(Const),
}


/// A unique identifier assigned to each value node in the graph. These IDs are
/// wrapped specific ValueRef variants to refer to values in the graph.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ValueId(usize);

impl ValueId {
	/// Allocate a new unique value ID.
	pub fn alloc() -> ValueId {
		ValueId(NEXT_VALUE_ID.fetch_add(1, Ordering::SeqCst) + 1)
	}

	/// Get the underlying integer ID.
	pub fn as_usize(self) -> usize {
		self.0
	}
}

impl std::fmt::Display for ValueId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.as_usize())
	}
}

/// The next ID to be allocated in `ValueId::alloc()`. Incremented atomically.
static NEXT_VALUE_ID: AtomicUsize = ATOMIC_USIZE_INIT;

/// The ID of inline values such as constants.
pub const INLINE_VALUE_ID: ValueId = ValueId(0);
// TODO: Maybe we want to get rid of this in favor of removing `id()` from the
// `Value` trait and adding it to a `HasId` trait. Where the ID is needed, we
// would use a `Value + HasId` bound.


/// Declares a new wrapper type around ValueRef, allowing the target of the
/// reference to be encoded in the type, e.g. `ArgumentRef` or `InstRef`.
macro_rules! declare_ref {
    ($name:ident, $variant:ident) => {
    	#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
		pub struct $name(ValueId);

		impl $name {
			pub fn new(id: ValueId) -> $name {
				$name(id)
			}
		}

		impl Into<ValueRef> for $name {
			fn into(self) -> ValueRef {
				ValueRef::$variant(self)
			}
		}

		impl Into<ValueId> for $name {
			fn into(self) -> ValueId {
				self.0
			}
		}

		impl std::fmt::Display for $name {
			fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(f, "{}", self.0)
			}
		}

		impl std::fmt::Debug for $name {
			fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(f, "{}({})", stringify!($name), self.0)
			}
		}
    }
}

declare_ref!(FunctionRef, Function);
declare_ref!(ProcessRef, Process);
declare_ref!(EntityRef, Entity);
declare_ref!(ArgumentRef, Argument);
declare_ref!(BlockRef, Block);
declare_ref!(InstRef, Inst);


/// A context is anything that can resolve the name and type of a ValueRef.
/// Contexts are expected to form a hierarchy, such that a context wrapping e.g.
/// a function falls back to a parent context wrapping the module if a value
/// cannot be appropriately resolved.
pub trait Context : AsContext {
	/// Try to resolve a `ValueRef` to an actual `&Value` reference. May fail if
	/// the value is not known to the context.
	fn try_value(&self, value: &ValueRef) -> Option<&Value>;

	/// Get the parent context to which value resolution shall escalate. May
	/// return `None` for the context at the top of the hierarchy.
	fn parent(&self) -> Option<&Context> {
		None
	}

	/// Resolve a `ValueRef` to an actual `&Value` reference. Panics if the
	/// value is unknown to this context and its parents.
	fn value(&self, value: &ValueRef) -> &Value {
		self.try_value(value).or_else(|| self.parent().map(|p| p.value(value))).unwrap()
	}

	/// Get the type of a value.
	fn ty(&self, value: &ValueRef) -> Type {
		self.value(value).ty()
	}

	/// Get the name of a value.
	fn name(&self, value: &ValueRef) -> Option<&str> {
		self.value(value).name()
	}
}

pub trait AsContext {
	fn as_context(&self) -> &Context;
}

impl<T: Context> AsContext for T {
	fn as_context(&self) -> &Context { self }
}
