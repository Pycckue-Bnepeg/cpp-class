pub use cpp_class_macro::*;

#[repr(C)]
pub struct GenericTable<Table: Sized, Ty: Sized + 'static> {
    pub offset: isize,
    pub type_info: &'static Ty,
    pub vtable: Table,
}

#[repr(C)]
pub struct BaseTypeInfo {
    pub vtable: usize,
    pub name: *const u8,
}

unsafe impl Send for BaseTypeInfo {}
unsafe impl Sync for BaseTypeInfo {}

#[repr(C)]
pub struct Base {
    pub base: &'static BaseTypeInfo,
    pub offset_flags: usize,
}

#[repr(C)]
pub struct MultipleBasesTypeInfo<const COUNT: usize> {
    pub vtable: usize,
    pub name: *const u8,
    pub flags: u32,
    pub bases_count: u32,
    pub bases: [Base; COUNT],
}

unsafe impl<const COUNT: usize> Send for MultipleBasesTypeInfo<COUNT> {}
unsafe impl<const COUNT: usize> Sync for MultipleBasesTypeInfo<COUNT> {}
