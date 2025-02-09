## Overview
This macro generates code for handling the state of an application composed of several models.

The model state altering funtions are interpreted in a CQRS style.
By using locks on the model the state can be altered concurrently.

This macro is made to work with [Flutter-Rust-Bridge](https://github.com/fzyzcjy/flutter_rust_bridge) (called FRB in the following), so that the UI can be implemented in Flutter,
while the logic and state is handled in Rust. However, this is not a strickt requirement.
The FRB generated `RustAutoOpaque`  type is in essence a `RWLock`.

Inspired by [Crux](https://redbadger.github.io/crux/) we call the app, which uses this macro, the `app core` and the calling app(s) `shell apps`. See [crux's explanation](https://redbadger.github.io/crux/#overview).

The macro will generate enums for each model behind such a lock. The enum variants represent function calls on the model.
To get data from rust to flutter, wich is NOT zero copy, a clone of the lock (which is cheap) is returned, on with the flutter code can aquire the RWLock (so we are thread safe) and use getters implemented on the model.

This [example project](https://github.com/patmuk/flutter-UI_rust-BE-example) demonstrates how to utilize this in your own project :)


## Quickstart or what you need to implement
This is a summary what you need to implement, and what the macro will generate. Read the following sections to understand the details and look into the [example project](https://github.com/patmuk/flutter-UI_rust-BE-example) for reference.

1. create a lifecycle.rs file and define a `struct LifecycleImpl`.
2. apply this macro (see below)
3. implement the supporting traits (generated by the macro) as singletons (copy them from the [example project](https://github.com/patmuk/flutter-UI_rust-BE-example)) and extend them to your needs).
   1. `ìmpl AppConfig for AppConfigImpl` - this struct should hold a reference to the place where the state gets persisted (e.g. a file location or database, that should be retrievable by calling `fn borrow_app_state_url(&self) -> &str`).
   It can contain other configuration data for your app, but note that it is not part of the app's state and thus will not be persisted (if you need this, implement the configuration as a model). After initialization it is immutable, changes on values will have no effect. It can be retrieved via the lifecycle-singleton anytime.
   2. `ìmpl AppState for AppStateImpl` - this struct should hold the app' state. The shell app should not access this struct directly. Access and modification to it's fields will be done by the CQRS functions.
   3. `impl AppStatePersister` and `AppStatePersistError` - this struct handels the persistance and retrieval of the app's state. Again, the shell app is not accessing this directly - the state is loaded when the lifecycle is initialized and persisted every time a CQRS command lead to an actual change of the model's data (the return value's boolean is `true`).
   4. extend the `impl lifecycle`. This is the struct the shell app will interact with. See below on how to implement it.
4. implement the models. See below how to do that.
   
## How to apply the macro
Add this macro as an attriute to your struct, which implements the `Lifecycle` trait, generated by this macro.
(This sounds like a hen-and-egg problem, but it resolves automatically.)
E.g.:
```
#[generate_api(
    "app_core/src/domain/todo_list.rs",
    "app_core/src/domain/todo_category.rs"
)]
impl Lifecycle for LifecycleImpl { (...)
```

### How to implement the Lifecycle
The lifecycle instance is the main access point for the shell app.
It holds the global state of the app (your `impl AppState`) and thus should be a singleton.

When using flutter-rust-bridge we cannot return References from functions (see below) - thus we couldn't have a constructor who returns a reference to a lifecycle instance.
Instead you need to implement `get_singleton()`, which returns a &'static Self. Note that flutter-rust-bridge will ignore the return type, generating a function `get_singleton() -> ()`. That is ok, we need this internally function in the generated code to access the global state.

There are different ways to implement a singleton - I used `static SINGLETON: OnceLock<LifecycleImpl> = OnceLock::new();`. Note that the singleton should be immutable - changes to the AppState (which is immutable as well) occur over the RWLock.

The `fn initialise` kickstarts the app: It should create the lifecycle singleton and load the app's state. There are two `initialise` functions: `fn initialise_with_app_config<AC: AppConfig + std::fmt::Debug>(app_config: AC) -> Result<&'static Self, Self::Error>` is considered the main implementation, which works with a generic `AppConfig`. As flutter-rust-bridge doesn't support generics you need to implement `fn initialise(app_state_url: Option<String>) -> Result<(), Self::Error>` as well. 

`fn persist()` should implement persisting the app with your `impl AppStatePersister`. 

Lastly, `fn shutdown()` should be called by the shell app when the app is quit and will persist the state one last time. Implement any clean-up calls here.

### How to implement the models
For each model, implement in one file per model:
1. A struct, which `impl CQRSModel` (import the `CQRSModel` trait from Lifecycle, where the macro generates the code to). This holds the fields which make up your model.
This struct is part of the global AppState. To be persistable it needs to implement (or derive from) `serde::Serialize` and `serde::Deserialize`.
2. Implement a struct `impl CQRSModelLock`. This holds the RWLock on your model. This should probably allways look like:
```
#[derive(Debug, Default, Clone)]
pub struct MyModelLock {
    pub model: RustAutoOpaque<MyModel>,
}
impl CqrsModelLock<MyModel> for MyModelLock {
    fn for_model(model: MyModel) -> Self {
    Self {
        model: RustAutoOpaque::new(model),
        }
    }
}
```
You need to implement `serde::Serialize` and `serde::Deserialize` here as well - which can't be derived as we don't want to serialize the lock's actual state. Instead implement this (or something better suited for you):
```
impl Serialize for MyModelLock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize the model , the dirty flag is always false after loading
        self.model.blocking_read().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MyModelLock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let model = MyModel::deserialize(deserializer)?;
        Ok(Self::for_model(model))
    }
}
```
When we serialize the model, we make sure via blocking_read() that no data can be written/all has been written. When deserializing, we create a new Lock.
The only reasons why this code is not generated is that you (1) might want to extend it and you (2) might want to use it with something else than `flutter-rust-bridge`, e.g. implement `RWLock<>` instead of `RustAutoOpaqu<>` (and (3) all code is generated to the lifecycle-containing file).
3. implement `pub enum MyModelEffect`, which serves as a message to the shell app to do something. This is typically anything only the shell app can do, like `MyModelEffect::NotifyTheUser`. Instead of unit enum variants you can specify payloads as well, which are sent to the shell app. Note that these have to be copied - thus avoid heavy data. Keep in mind that the shell app might not always want to have the latest data. For example, if you have a `fn delete_item -> MyModel::RenderItems`, the shell app might want to call this function several times before updating the list of (remaining) items. So, in most cases you want to return a copy of the lock only (`MyModel::RenderItems(MyModelLock)`), which the shell app can use later to get the list of items (e.g. `my_model_lock.model.blocking_read().get_items()`).
4. Implement CQRS commands and queries. The queries should return data (without side effects), while only the commands should modify the app's state. Implement them on the Lock struct (e.g. `impl MyMoLock {`).
They have to have a reference to `&self` and can have any additional parameters. The return type of the CQRS queries has to be `Result<Vec<MyModelEffect>, MyModelProcessingError>` and `Result<(bool, Vec<MyModelEffect>), MyModelProcessingError>` for CQRS commands.
Set the boolean to `false`, if the state did not change and to `true` otherwise. If set to true the state will be automatically persisted (by the generated code).
5. Implement `pub enum MyModelProcessingError`. Specify any error as enum variants and use them in the CQRS function implementations. Use `thiserror` to easily implement meaningfull error (see below for more).
6. Implement getters on the model (like `impl MyModel {`). As explained, using these the shell app can retrieve updates to the model when it needs them, and extract only the attributes needed. Often the complete model is too large, and rarely needed completely by the shell app. You can, of course, combine several attributes or derived data (like the number of items instead a full list of items) in a struct returned by such a function (which is known as a view-model).

## How to call the generated api
The main work the macro does is combining all CQRS calls of all models into one structure. Thus, on the rust side each model's functions, locks, effects and errors can be defined separate while the shell app can call these centralized, making for example error handling much more convinient.

### how to initialize the lifecycle
The shell app interacts with `impl lifecycle` only. Depending on the shell apps nature (e.g. rust or Flutter), the functions in `lifecycle` are called directly or via a bridge (e.g. FRB, after code generation).
I wrote this macro mainly to interact with a flutter shell app, connected using FRB. However, this description will have a rust shell app in mind.

In the [example project](https://github.com/patmuk/flutter-UI_rust-BE-example) you can find a [rust shell](https://github.com/patmuk/flutter-UI_rust-BE-example/blob/main/shell_cli/src/main.rs) and a [flutter shell](https://github.com/patmuk/flutter-UI_rust-BE-example/tree/main/shell_flutter) implementation.

Start by calling `Lifecycle::initialise`, either by providing an instance of `impl AppConfig` or the url where the app's state is/will be stored as a `String`. In the latter case, an instance of `impl AppConfig` will be generated.
This call loads (or creates) the app's state.

### how to call the CQRS functions (commands and queries)
The generated code contains one enum for all CQRS commands and one enum for all CQRS queries of a model.
These implement the CQRS trait, which enables calling `process()` on them.

For example, to call the command `add_item(String)` which is implemented for your `impl CQRSModelLock<MyModel> for MyModelLock` call process on the generated enum variant `MyModelCommand::AddItem("new item".to_string())`.

You will receive a `Result<(bool, Vec<TodoListEffect>), TodoListProcessingError>` for a command and a `Result<Vec<TodoListEffect>, TodoListProcessingError>` for a query. The boolean signals that the app's state actually changed (n.b. you might implement `add_item` to ignore subsequent calls with the same content).

I recommend handling the returned value in a single function, so that the Effects are processed the same way each time (DRY) (See `fn process_and_handle_effects` in the [rust shell app example](https://github.com/patmuk/flutter-UI_rust-BE-example/blob/main/shell_cli/src/main.rs)).

Additionally you should update the view model in the shell app (see `Future<Void> handle_effects` in the [flutter shell example](https://github.com/patmuk/flutter-UI_rust-BE-example/blob/main/shell_flutter/lib/state_handler.dart))

### what else to implement?
When your shell-app terminates call `Lifecycle::shutdown()`. As the app can crash you should not rely on this call. This cann should do anything needed to gracefully shut down the app. In the example implementation it persists the app's state a final time (although this should not be needed, as it is persisted after every change). You might want to close any open db connections - though most rust crates do that on drop() automatically. Nevertheless, since you implemented `Lifecycle::shutdown` you should not for get to call it (but not rely on that call happening (app crash) either).

## traits
The macro generates some (more) traits, which need to be implemented by you.
These traits are not generated, but copies of the sourceCode in `generate_cqrs_api_macro_impl/src/generating/traits`.
- Lifecycle - as the general API interface, to be consumed by other apps
- AppConfig - structure to hold the apps configuration, like the path to the persisting file
- AppState - struct to hold the app's state
- AppStatePersister - implementation to persist and retrieve the app's state, e.g. into a file
- AppStatePersistError - marker for the Error your AppStatePersister implementation can return

Each of your model implementations need to implement these traits
- CQRSModel - marker so that the code generation recognizes your model
- CQRSModelLock - marker so that the lock to the model is recognized
- CQRS - this trait marks CQRS commands and queries. This is implemented automatically.

The used traits are generated from static files in `generate_cqrs_api_macro_impl/src/generating/traits`.
This makes them available in the `impl Lifecycle` for the rest of the using codebase. 

#### implement your models
Each model needs to be composed of two parts:
1. a struct holding the model's data (`impl CQRSModel`) and
2. a struct implementing the model's functions and holding a lock to the model (`impl CQRSModelLock`).

These structs and their fields should be public accessible, so that a shell app can use them.

#### implement functions on your model
The functions, which either manipulate the model's state (CQRS commands) or return the model's value(s) (CQRS queries) are to be implemented in your `CQRSModelLock` struct.

They are recognized for code generation by their signature:

`-> Result<Vec<Effect>, ProcessingError>` for CQRS queries and
`-> Result<(bool, Vec<Effect>), ProcessingError>` for CQRS commands.

The generated code will automatically implement the CQRS trait.

Functions with other return types are ignored.

The CQRS functions should be crate-private (pub(crate)), to not pollute your api. Shell apps are accessing them via functions generated by the macro and placed in your lifecycle file.
Non-CQRS functions and functions on the `impl CQRSModel` should be prublic, if you want shell apps to access them. The macro will ignore them. I recommend to keep the functions in `impl CQRSModelLock` private and the getter functions in `impl CQRSModel` public. If you follow the recommended pattern described here the CQRSQueries will return a reference to the lock only, from where shell apps will pull the needed data using getters in `impl CQRSModel`.

Access the model via the lock, which is your `Self` object, by either calling `.blocking_read()` for read-only access or `blocking_write()` for write-only access.
In harmony with rust's borrowing concept, there can be multiple reads but only a single write.
Calling this function blocks the control flow until the lock is aquired.

##### restrictions
Because of current limitations in flutter-rust-bridge as well as this marco's implementation, you need to take care of the following limitations. These can be lifted in the future - however, as of now there was no need to do so. If you need it feel free to create an issue and/or submit a pull-request!

###### Don't use references (all data send from Rust to Flutter needs to be .clone())
In the api function implementations (`impl CQRSModelLock`) don't use references for parameters or return values.
As data is passed between rust and flutter, supporting references would be an enormous undertaking ("lifetime", "concurrency", ...).

If you do so the generate code will incorrectly generate an enum variant, which is missing the parameters that are a reference type.

However, data passed from Flutter to Rust (i.e. the function parameters) is zero copy.
Data passed from Rust to Flutter (i.e. return values) costs, and `clone()` must be used. Thus it is best practise to not return the complete model, but only the lock on the model (which practically serves as a reference). The model shall implement specific getters, which are called from the Flutter side after aquiring the read lock. Because the caller might not always want to access this specific data we don't send it proactively. However, this last implementation detail is up to you! 

###### CQRS-Commands and -Queries need to be implemented
You will get the error "function .map_err() doesn't exist" (or something similar) if you did not implemented at least one Command and at least on Query function. In the generated code you will see `match self {}.map_err`.


##### Implementing Errors
Any possible Error should be defined in an `enum Error` and can be returned from your cqrs function. Do this per model - the macro takes care to combine it to one `enum Error` for the caller.
I reccomend to utilize `thiserror`.
For example, if you provide in your my_model.rs:
```
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MyModelError {
    #[error("I am not a model!")]
    CannotModelError,
    #[error("This model belongs to {0}!")]
    NotMyModelError(String)
}
```
and in your my_other_model.rs
```
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MyOtherModelError {
    #[error("I am the wrong!")]
    WrongModelError
}
```
the macro will generate
```
use crate::domain::models::MyModelError;
use crate::domain::models::MyOtherModelError;
#[derive(thiserror::Error, Debug)]
pub enum ProcessingError {
    #[error("I am not a model!")]
    MyModelCannotModelError,
    #[error("This model belongs to {0}!")]
    MyModelNotMyModelError(String)
    #[error("I am the wrong!")]
    MyOtherModelWrongModelError
}
```
and take care of the necessary conversions. 

##### Implementing Effects
Following the event driven philosophy any function leads to an Effect, which is a message to the shell app to do something.
This can be anything, typically it is asking the shell app to render some values.

For example, a CQRSCommand mit delete an entry and require the shell app to notify the user about this (e.g. MyModelEffect::NotifyUser(String)).

For CQRSQueries the effect is typically a request grab some data (e.g. MyModelEffect::Render(MyModelLock)).

Define possible effects for your model in an `enum Effect`.
While CQRS queries don't alter a model's state, they can have an effect on the shell app.
And one CQRS functon can have multiple effects.

Thus, we return `Vec<Effect>`.

Similar to the Errors, the macro will combine the effects defined for each model into one enum.

##### Return model values
There is two ways to return a model's values: 1. returning the value directly or 2. returning the whole model.

If the value is not heavy it can be returned directly. For this, implement an `enum Effect` variant, that can hold your value, like `RenderText(String)`.
This value has to be `clone()`ed. Note that for FRB all structs of these values have to be non-opaque. Otherwise the whole (generated) Effect enum will be Opaque and no data can be returned.

Alternatively you can return the whole model - or more preceise the lock on the value: Have an `enum Effect` variant, which holds the model's lock, like `RenderModel(ModelLock)`.
Again, you need to `clone()` the lock - but this is very lightweight (depending on the nature of your consuming app. If it is a Rust app or a flutter app, connected by FRB, it is only copying a pointer).

Now the shell app can aquire the lock on the model and read the needed values.

Note that this, only sending a reference to the model (the lock) instead of the whole model, is possilbe when using FRB by making it opaque: The lock is a `RustAutoOpaque<MyModel>`. Thus, the model cannot be a field of an enum variant directly.

However, do not write on the model, as this would not be detected by the state management!
Therefore, keep the model's fields private and expose them only via getters (in addition to the CQRS Queries).

I recommend to combine both: Always return the `ModelLock` and the minimal needed data, so the shell app can call a getter function on the model when needed. For example, if you have a list of models, each having a model_id: A `QueryModel` call would return the found model_id along with the `ModelLock`. The shell app can now get a read-lock on the model (`ModelLock.blocking_read().model`) and call the getter `model.get_model_by_id(model_id)`. 

The getters on the model should be streight forward - at maximum you can create a view on the model, but refrain from implementing more complex logic. This should be done in the CQRS functions, so the shell app has more control when to run these, potentially longer running methods (without holding a lock).

Remember that the shell app should not do any data manipulation on the model directly on the retrieved data from effects or getters on the model. These data are (fresh) copies only and won't change the app's state!

A setter function from the model could change data in the app's state - but unless you re-implement the automatically generated persistance mechanism these changes are not saved immediately and might get lost. So leave alterations to the CQRSCommands!

### Configure the app's state management
Don't forget to add each model as a field into your implementation of the `AppState` trait.

#### when RustAutoOpaque<MyModel> is generated?
After adding your model as a field to your `AppState` implementation the next FRB code generation run will pick it up and generate your `RustAutoOpaque<MyModel>` struct.

The recommended order is:

1. Implement your CQRSModel and CQRSModelLock implementation. Here the Lock needs to have a field of type `RustAutoOpaque<MyModel>`, for which the compiler will complain it doesn't exist.
2. Add the Lock as a field to your `AppState` implementation.
3. Run the FRB code generation.
Now `RustAutoOpaque<MyModel>` is generated.
