pub use cpp_class_macro::*;

#[repr(C)]
pub struct GenericTable<Table: Sized, Ty: Sized + 'static> {
    pub offset: isize,
    pub type_info: &'static Ty,
    pub vtable: Table,
}

#[repr(C)]
pub struct BaseTypeInfo {
    pub vtable: &'static u8,
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
    pub vtable: &'static u8,
    pub name: *const u8,
    pub flags: u32,
    pub bases_count: u32,
    pub bases: [Base; COUNT],
}

#[repr(C)]
pub struct CppTi {
    base_vtable: usize,
    ti: usize,
    pub vtable: u8,
}

unsafe impl<const COUNT: usize> Send for MultipleBasesTypeInfo<COUNT> {}
unsafe impl<const COUNT: usize> Sync for MultipleBasesTypeInfo<COUNT> {}

extern "C" {
    #[link_name = "_ZTVN10__cxxabiv121__vmi_class_type_infoE"]
    pub static vmi_class_type_info: CppTi;

    #[link_name = "_ZTVN10__cxxabiv117__class_type_infoE"]
    pub static class_type_info: CppTi;
}

// _ZTVN10__cxxabiv121__vmi_class_type_infoE@@CXXABI_1.3
// _ZTVN10__cxxabiv117__class_type_infoE@@CXXABI_1.3
