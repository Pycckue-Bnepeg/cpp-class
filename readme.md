## cpp-class
adds ability to create structs and traits compatible with c++ virtual classes.

currently supports only multiple inheritance classes without any data. only virtual functions.

gcc only?

## example
this has been written to support ragemp cpp sdk to develop plugins.

sdk repo at: https://github.com/ragemultiplayer/ragemp-cppsdk

```c++
class IEventHandler
{
    public:
        virtual IEntityHandler *GetEntityHandler();
        virtual IPlayerHandler *GetPlayerHandler();
        virtual IVehicleHandler *GetVehicleHandler();
        virtual IColshapeHandler *GetColshapeHandler();
        virtual ICheckpointHandler *GetCheckpointHandler();
        virtual IMarkerHandler *GetMarkerHandler();
        virtual IPickupHandler *GetPickupHandler();
        virtual ITickHandler *GetTickHandler();
        virtual ILocalEventHandler *GetLocalEventHandler();
        virtual IConnectionHandler *GetConnectionHandler();
        virtual IDebugHandler *GetDebugHandler();
        virtual IServerHandler *GetServerHandler();
        virtual IRpcHandler *GetRpcHandler();
};

class ITickHandler
{
    public:
        virtual void Tick();
};

// it would be like
class EventHandler: public IEventHandler, public ITickHandler {
    public:
        virtual ITickHandler *GetTickHandler() {
            return this;
        }

        // .....

        virtual void Tick() { std::cout << "tick!!!!" << std::endl; }
}

RAGE_API rage::IPlugin *InitializePlugin(rage::IMultiplayer *mp)
{
	mp->AddEventHandler(new EventHandler);
	return new rage::IPlugin;
}
```

rust version

```rust
use cpp_class::vtable;

// supports this:
// __cxxabiv1::__vmi_class_type_info
// __cxxabiv1::__class_type_info
#[vtable]
pub mod handler {

    // list of parents, it should be 2+
    // your struct can contain any data with any repr
    #[vtable::derive(IEventHandler, ITickHandler)]
    pub struct Handler {
        pub data_1: usize,
        pub data_2: u32,
    }

    // default abi is fastcall
    // type_name is a name of a class name defined at headers (or in a executable) (nul char is append by the macro)
    // no data
    #[vtable::virtual_class(abi = fastcall, link_name = "N4rage13IEventHandlerE")]
    trait IEventHandler {
        // no default impl at this moment
        // also destructors are not supported
        // returns bool because the return value is not used by ragemp (simple check if it is not 0)
        fn entity_handler(&mut self) -> bool;
        fn player_handler(&mut self) -> bool;
        fn vehicle_handler(&mut self) -> bool;
        fn colshape_handler(&mut self) -> bool;
        fn checkpoint_handler(&mut self) -> bool;
        fn unk_0(&mut self) -> bool;
        fn unk_1(&mut self) -> bool;
        fn tick_handler(&mut self) -> bool;
        fn local_event_handler(&mut self) -> bool;
        fn connection_handler(&mut self) -> bool;
        fn debug_handler(&mut self) -> bool;
        fn server_handler(&mut self) -> bool;
        fn rpc_handler(&mut self) -> bool;
    }

    #[vtable::virtual_class(abi = fastcall, link_name = "N4rage12ITickHandlerE")]
    trait ITickHandler {
        fn tick(&mut self);
    }

    impl IEventHandler for Handler {
        fn tick_handler(&mut self) -> bool {
            true
        }

        // ...
    }

    impl ITickHandler for Handler {
        fn tick(&mut self) {
            println!("tick! value {:X} prev {}", self.data_1, self.data_2);
            self.data_2 += 1;
            println!("new {}", self.data_2);
        }
    }
}

// definition of rage::PluginManager is omitted
#[no_mangle]
pub extern "C" fn InitializePlugin(mp: *mut rage::PluginManager) -> u64 {
    let object = handler::Handler {
        data_1: 0xAABBCCDD,
        data_2: 0,
    };

    // cpp_class::vtable generates make_boxed and from_boxed functions
    // from_boxed should be used to destroy an object
    // returns `RefHandler` pointer that contains vtables and points at the start
    let raw = handler::make_boxed(object);

    unsafe {
        ((*(*mp).vftable).add_event_handler)(mp, raw as *mut _);
    }

    return 1;
}

```
