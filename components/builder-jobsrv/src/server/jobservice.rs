// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

use protobuf::Message as Message_imported_for_functions;
use protobuf::ProtobufEnum as ProtobufEnum_imported_for_functions;

#[derive(PartialEq,Clone,Default)]
pub struct HelloRequest {
    // message fields
    pub greeting: ::std::string::String,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for HelloRequest {}

impl HelloRequest {
    pub fn new() -> HelloRequest {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static HelloRequest {
        static mut instance: ::protobuf::lazy::Lazy<HelloRequest> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const HelloRequest,
        };
        unsafe {
            instance.get(HelloRequest::new)
        }
    }

    // string greeting = 1;

    pub fn clear_greeting(&mut self) {
        self.greeting.clear();
    }

    // Param is passed by value, moved
    pub fn set_greeting(&mut self, v: ::std::string::String) {
        self.greeting = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_greeting(&mut self) -> &mut ::std::string::String {
        &mut self.greeting
    }

    // Take field
    pub fn take_greeting(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.greeting, ::std::string::String::new())
    }

    pub fn get_greeting(&self) -> &str {
        &self.greeting
    }

    fn get_greeting_for_reflect(&self) -> &::std::string::String {
        &self.greeting
    }

    fn mut_greeting_for_reflect(&mut self) -> &mut ::std::string::String {
        &mut self.greeting
    }
}

impl ::protobuf::Message for HelloRequest {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.greeting)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if !self.greeting.is_empty() {
            my_size += ::protobuf::rt::string_size(1, &self.greeting);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if !self.greeting.is_empty() {
            os.write_string(1, &self.greeting)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &::std::any::Any {
        self as &::std::any::Any
    }
    fn as_any_mut(&mut self) -> &mut ::std::any::Any {
        self as &mut ::std::any::Any
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<::std::any::Any> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for HelloRequest {
    fn new() -> HelloRequest {
        HelloRequest::new()
    }

    fn descriptor_static(_: ::std::option::Option<HelloRequest>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "greeting",
                    HelloRequest::get_greeting_for_reflect,
                    HelloRequest::mut_greeting_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<HelloRequest>(
                    "HelloRequest",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for HelloRequest {
    fn clear(&mut self) {
        self.clear_greeting();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for HelloRequest {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for HelloRequest {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct HelloResponse {
    // message fields
    pub reply: ::std::string::String,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for HelloResponse {}

impl HelloResponse {
    pub fn new() -> HelloResponse {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static HelloResponse {
        static mut instance: ::protobuf::lazy::Lazy<HelloResponse> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const HelloResponse,
        };
        unsafe {
            instance.get(HelloResponse::new)
        }
    }

    // string reply = 1;

    pub fn clear_reply(&mut self) {
        self.reply.clear();
    }

    // Param is passed by value, moved
    pub fn set_reply(&mut self, v: ::std::string::String) {
        self.reply = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_reply(&mut self) -> &mut ::std::string::String {
        &mut self.reply
    }

    // Take field
    pub fn take_reply(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.reply, ::std::string::String::new())
    }

    pub fn get_reply(&self) -> &str {
        &self.reply
    }

    fn get_reply_for_reflect(&self) -> &::std::string::String {
        &self.reply
    }

    fn mut_reply_for_reflect(&mut self) -> &mut ::std::string::String {
        &mut self.reply
    }
}

impl ::protobuf::Message for HelloResponse {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.reply)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if !self.reply.is_empty() {
            my_size += ::protobuf::rt::string_size(1, &self.reply);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if !self.reply.is_empty() {
            os.write_string(1, &self.reply)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &::std::any::Any {
        self as &::std::any::Any
    }
    fn as_any_mut(&mut self) -> &mut ::std::any::Any {
        self as &mut ::std::any::Any
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<::std::any::Any> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for HelloResponse {
    fn new() -> HelloResponse {
        HelloResponse::new()
    }

    fn descriptor_static(_: ::std::option::Option<HelloResponse>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "reply",
                    HelloResponse::get_reply_for_reflect,
                    HelloResponse::mut_reply_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<HelloResponse>(
                    "HelloResponse",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for HelloResponse {
    fn clear(&mut self) {
        self.clear_reply();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for HelloResponse {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for HelloResponse {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\x10jobservice.proto\x12\njobservice\"*\n\x0cHelloRequest\x12\x1a\n\
    \x08greeting\x18\x01\x20\x01(\tR\x08greeting\"%\n\rHelloResponse\x12\x14\
    \n\x05reply\x18\x01\x20\x01(\tR\x05reply2M\n\nJobService\x12?\n\x08SayHe\
    llo\x12\x18.jobservice.HelloRequest\x1a\x19.jobservice.HelloResponseJ\
    \xa7\x02\n\x06\x12\x04\0\0\x0e\x01\n\x08\n\x01\x0c\x12\x03\0\0\x12\n\x08\
    \n\x01\x02\x12\x03\x02\x08\x12\n\n\n\x02\x06\0\x12\x04\x04\0\x06\x01\n\n\
    \n\x03\x06\0\x01\x12\x03\x04\x08\x12\n\x0b\n\x04\x06\0\x02\0\x12\x03\x05\
    \x026\n\x0c\n\x05\x06\0\x02\0\x01\x12\x03\x05\x06\x0e\n\x0c\n\x05\x06\0\
    \x02\0\x02\x12\x03\x05\x10\x1c\n\x0c\n\x05\x06\0\x02\0\x03\x12\x03\x05'4\
    \n\n\n\x02\x04\0\x12\x04\x08\0\n\x01\n\n\n\x03\x04\0\x01\x12\x03\x08\x08\
    \x14\n\x0b\n\x04\x04\0\x02\0\x12\x03\t\x02\x16\n\r\n\x05\x04\0\x02\0\x04\
    \x12\x04\t\x02\x08\x16\n\x0c\n\x05\x04\0\x02\0\x05\x12\x03\t\x02\x08\n\
    \x0c\n\x05\x04\0\x02\0\x01\x12\x03\t\t\x11\n\x0c\n\x05\x04\0\x02\0\x03\
    \x12\x03\t\x14\x15\n\n\n\x02\x04\x01\x12\x04\x0c\0\x0e\x01\n\n\n\x03\x04\
    \x01\x01\x12\x03\x0c\x08\x15\n\x0b\n\x04\x04\x01\x02\0\x12\x03\r\x02\x13\
    \n\r\n\x05\x04\x01\x02\0\x04\x12\x04\r\x02\x0c\x17\n\x0c\n\x05\x04\x01\
    \x02\0\x05\x12\x03\r\x02\x08\n\x0c\n\x05\x04\x01\x02\0\x01\x12\x03\r\t\
    \x0e\n\x0c\n\x05\x04\x01\x02\0\x03\x12\x03\r\x11\x12b\x06proto3\
";

static mut file_descriptor_proto_lazy: ::protobuf::lazy::Lazy<::protobuf::descriptor::FileDescriptorProto> = ::protobuf::lazy::Lazy {
    lock: ::protobuf::lazy::ONCE_INIT,
    ptr: 0 as *const ::protobuf::descriptor::FileDescriptorProto,
};

fn parse_descriptor_proto() -> ::protobuf::descriptor::FileDescriptorProto {
    ::protobuf::parse_from_bytes(file_descriptor_proto_data).unwrap()
}

pub fn file_descriptor_proto() -> &'static ::protobuf::descriptor::FileDescriptorProto {
    unsafe {
        file_descriptor_proto_lazy.get(|| {
            parse_descriptor_proto()
        })
    }
}
