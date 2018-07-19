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
    greeting: ::protobuf::SingularField<::std::string::String>,
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

    // optional string greeting = 1;

    pub fn clear_greeting(&mut self) {
        self.greeting.clear();
    }

    pub fn has_greeting(&self) -> bool {
        self.greeting.is_some()
    }

    // Param is passed by value, moved
    pub fn set_greeting(&mut self, v: ::std::string::String) {
        self.greeting = ::protobuf::SingularField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_greeting(&mut self) -> &mut ::std::string::String {
        if self.greeting.is_none() {
            self.greeting.set_default();
        }
        self.greeting.as_mut().unwrap()
    }

    // Take field
    pub fn take_greeting(&mut self) -> ::std::string::String {
        self.greeting.take().unwrap_or_else(|| ::std::string::String::new())
    }

    pub fn get_greeting(&self) -> &str {
        match self.greeting.as_ref() {
            Some(v) => &v,
            None => "",
        }
    }

    fn get_greeting_for_reflect(&self) -> &::protobuf::SingularField<::std::string::String> {
        &self.greeting
    }

    fn mut_greeting_for_reflect(&mut self) -> &mut ::protobuf::SingularField<::std::string::String> {
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
                    ::protobuf::rt::read_singular_string_into(wire_type, is, &mut self.greeting)?;
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
        if let Some(ref v) = self.greeting.as_ref() {
            my_size += ::protobuf::rt::string_size(1, &v);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if let Some(ref v) = self.greeting.as_ref() {
            os.write_string(1, &v)?;
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
                fields.push(::protobuf::reflect::accessor::make_singular_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
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
    reply: ::protobuf::SingularField<::std::string::String>,
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

    // optional string reply = 1;

    pub fn clear_reply(&mut self) {
        self.reply.clear();
    }

    pub fn has_reply(&self) -> bool {
        self.reply.is_some()
    }

    // Param is passed by value, moved
    pub fn set_reply(&mut self, v: ::std::string::String) {
        self.reply = ::protobuf::SingularField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_reply(&mut self) -> &mut ::std::string::String {
        if self.reply.is_none() {
            self.reply.set_default();
        }
        self.reply.as_mut().unwrap()
    }

    // Take field
    pub fn take_reply(&mut self) -> ::std::string::String {
        self.reply.take().unwrap_or_else(|| ::std::string::String::new())
    }

    pub fn get_reply(&self) -> &str {
        match self.reply.as_ref() {
            Some(v) => &v,
            None => "",
        }
    }

    fn get_reply_for_reflect(&self) -> &::protobuf::SingularField<::std::string::String> {
        &self.reply
    }

    fn mut_reply_for_reflect(&mut self) -> &mut ::protobuf::SingularField<::std::string::String> {
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
                    ::protobuf::rt::read_singular_string_into(wire_type, is, &mut self.reply)?;
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
        if let Some(ref v) = self.reply.as_ref() {
            my_size += ::protobuf::rt::string_size(1, &v);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if let Some(ref v) = self.reply.as_ref() {
            os.write_string(1, &v)?;
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
                fields.push(::protobuf::reflect::accessor::make_singular_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
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

#[derive(PartialEq,Clone,Default)]
pub struct JobGraphPackageStatsGet {
    // message fields
    origin: ::protobuf::SingularField<::std::string::String>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for JobGraphPackageStatsGet {}

impl JobGraphPackageStatsGet {
    pub fn new() -> JobGraphPackageStatsGet {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static JobGraphPackageStatsGet {
        static mut instance: ::protobuf::lazy::Lazy<JobGraphPackageStatsGet> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const JobGraphPackageStatsGet,
        };
        unsafe {
            instance.get(JobGraphPackageStatsGet::new)
        }
    }

    // optional string origin = 1;

    pub fn clear_origin(&mut self) {
        self.origin.clear();
    }

    pub fn has_origin(&self) -> bool {
        self.origin.is_some()
    }

    // Param is passed by value, moved
    pub fn set_origin(&mut self, v: ::std::string::String) {
        self.origin = ::protobuf::SingularField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_origin(&mut self) -> &mut ::std::string::String {
        if self.origin.is_none() {
            self.origin.set_default();
        }
        self.origin.as_mut().unwrap()
    }

    // Take field
    pub fn take_origin(&mut self) -> ::std::string::String {
        self.origin.take().unwrap_or_else(|| ::std::string::String::new())
    }

    pub fn get_origin(&self) -> &str {
        match self.origin.as_ref() {
            Some(v) => &v,
            None => "",
        }
    }

    fn get_origin_for_reflect(&self) -> &::protobuf::SingularField<::std::string::String> {
        &self.origin
    }

    fn mut_origin_for_reflect(&mut self) -> &mut ::protobuf::SingularField<::std::string::String> {
        &mut self.origin
    }
}

impl ::protobuf::Message for JobGraphPackageStatsGet {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_singular_string_into(wire_type, is, &mut self.origin)?;
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
        if let Some(ref v) = self.origin.as_ref() {
            my_size += ::protobuf::rt::string_size(1, &v);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if let Some(ref v) = self.origin.as_ref() {
            os.write_string(1, &v)?;
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

impl ::protobuf::MessageStatic for JobGraphPackageStatsGet {
    fn new() -> JobGraphPackageStatsGet {
        JobGraphPackageStatsGet::new()
    }

    fn descriptor_static(_: ::std::option::Option<JobGraphPackageStatsGet>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_singular_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "origin",
                    JobGraphPackageStatsGet::get_origin_for_reflect,
                    JobGraphPackageStatsGet::mut_origin_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<JobGraphPackageStatsGet>(
                    "JobGraphPackageStatsGet",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for JobGraphPackageStatsGet {
    fn clear(&mut self) {
        self.clear_origin();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for JobGraphPackageStatsGet {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for JobGraphPackageStatsGet {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct JobGraphPackageStats {
    // message fields
    plans: ::std::option::Option<u64>,
    builds: ::std::option::Option<u64>,
    unique_packages: ::std::option::Option<u64>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for JobGraphPackageStats {}

impl JobGraphPackageStats {
    pub fn new() -> JobGraphPackageStats {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static JobGraphPackageStats {
        static mut instance: ::protobuf::lazy::Lazy<JobGraphPackageStats> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const JobGraphPackageStats,
        };
        unsafe {
            instance.get(JobGraphPackageStats::new)
        }
    }

    // optional uint64 plans = 1;

    pub fn clear_plans(&mut self) {
        self.plans = ::std::option::Option::None;
    }

    pub fn has_plans(&self) -> bool {
        self.plans.is_some()
    }

    // Param is passed by value, moved
    pub fn set_plans(&mut self, v: u64) {
        self.plans = ::std::option::Option::Some(v);
    }

    pub fn get_plans(&self) -> u64 {
        self.plans.unwrap_or(0)
    }

    fn get_plans_for_reflect(&self) -> &::std::option::Option<u64> {
        &self.plans
    }

    fn mut_plans_for_reflect(&mut self) -> &mut ::std::option::Option<u64> {
        &mut self.plans
    }

    // optional uint64 builds = 2;

    pub fn clear_builds(&mut self) {
        self.builds = ::std::option::Option::None;
    }

    pub fn has_builds(&self) -> bool {
        self.builds.is_some()
    }

    // Param is passed by value, moved
    pub fn set_builds(&mut self, v: u64) {
        self.builds = ::std::option::Option::Some(v);
    }

    pub fn get_builds(&self) -> u64 {
        self.builds.unwrap_or(0)
    }

    fn get_builds_for_reflect(&self) -> &::std::option::Option<u64> {
        &self.builds
    }

    fn mut_builds_for_reflect(&mut self) -> &mut ::std::option::Option<u64> {
        &mut self.builds
    }

    // optional uint64 unique_packages = 3;

    pub fn clear_unique_packages(&mut self) {
        self.unique_packages = ::std::option::Option::None;
    }

    pub fn has_unique_packages(&self) -> bool {
        self.unique_packages.is_some()
    }

    // Param is passed by value, moved
    pub fn set_unique_packages(&mut self, v: u64) {
        self.unique_packages = ::std::option::Option::Some(v);
    }

    pub fn get_unique_packages(&self) -> u64 {
        self.unique_packages.unwrap_or(0)
    }

    fn get_unique_packages_for_reflect(&self) -> &::std::option::Option<u64> {
        &self.unique_packages
    }

    fn mut_unique_packages_for_reflect(&mut self) -> &mut ::std::option::Option<u64> {
        &mut self.unique_packages
    }
}

impl ::protobuf::Message for JobGraphPackageStats {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.plans = ::std::option::Option::Some(tmp);
                },
                2 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.builds = ::std::option::Option::Some(tmp);
                },
                3 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.unique_packages = ::std::option::Option::Some(tmp);
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
        if let Some(v) = self.plans {
            my_size += ::protobuf::rt::value_size(1, v, ::protobuf::wire_format::WireTypeVarint);
        }
        if let Some(v) = self.builds {
            my_size += ::protobuf::rt::value_size(2, v, ::protobuf::wire_format::WireTypeVarint);
        }
        if let Some(v) = self.unique_packages {
            my_size += ::protobuf::rt::value_size(3, v, ::protobuf::wire_format::WireTypeVarint);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if let Some(v) = self.plans {
            os.write_uint64(1, v)?;
        }
        if let Some(v) = self.builds {
            os.write_uint64(2, v)?;
        }
        if let Some(v) = self.unique_packages {
            os.write_uint64(3, v)?;
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

impl ::protobuf::MessageStatic for JobGraphPackageStats {
    fn new() -> JobGraphPackageStats {
        JobGraphPackageStats::new()
    }

    fn descriptor_static(_: ::std::option::Option<JobGraphPackageStats>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "plans",
                    JobGraphPackageStats::get_plans_for_reflect,
                    JobGraphPackageStats::mut_plans_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "builds",
                    JobGraphPackageStats::get_builds_for_reflect,
                    JobGraphPackageStats::mut_builds_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "unique_packages",
                    JobGraphPackageStats::get_unique_packages_for_reflect,
                    JobGraphPackageStats::mut_unique_packages_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<JobGraphPackageStats>(
                    "JobGraphPackageStats",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for JobGraphPackageStats {
    fn clear(&mut self) {
        self.clear_plans();
        self.clear_builds();
        self.clear_unique_packages();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for JobGraphPackageStats {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for JobGraphPackageStats {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\x10jobservice.proto\x12\njobservice\"*\n\x0cHelloRequest\x12\x1a\n\
    \x08greeting\x18\x01\x20\x01(\tR\x08greeting\"%\n\rHelloResponse\x12\x14\
    \n\x05reply\x18\x01\x20\x01(\tR\x05reply\"1\n\x17JobGraphPackageStatsGet\
    \x12\x16\n\x06origin\x18\x01\x20\x01(\tR\x06origin\"m\n\x14JobGraphPacka\
    geStats\x12\x14\n\x05plans\x18\x01\x20\x01(\x04R\x05plans\x12\x16\n\x06b\
    uilds\x18\x02\x20\x01(\x04R\x06builds\x12'\n\x0funique_packages\x18\x03\
    \x20\x01(\x04R\x0euniquePackages2\xaf\x01\n\nJobService\x12?\n\x08SayHel\
    lo\x12\x18.jobservice.HelloRequest\x1a\x19.jobservice.HelloResponse\x12`\
    \n\x17GetJobGraphPackageStats\x12#.jobservice.JobGraphPackageStatsGet\
    \x1a\x20.jobservice.JobGraphPackageStatsJ\xa0\x05\n\x06\x12\x04\0\0\x19\
    \x01\n\x08\n\x01\x0c\x12\x03\0\0\x12\n\x08\n\x01\x02\x12\x03\x02\x08\x12\
    \n\n\n\x02\x06\0\x12\x04\x04\0\x07\x01\n\n\n\x03\x06\0\x01\x12\x03\x04\
    \x08\x12\n\x0b\n\x04\x06\0\x02\0\x12\x03\x05\x026\n\x0c\n\x05\x06\0\x02\
    \0\x01\x12\x03\x05\x06\x0e\n\x0c\n\x05\x06\0\x02\0\x02\x12\x03\x05\x10\
    \x1c\n\x0c\n\x05\x06\0\x02\0\x03\x12\x03\x05'4\n\x0b\n\x04\x06\0\x02\x01\
    \x12\x03\x06\x02W\n\x0c\n\x05\x06\0\x02\x01\x01\x12\x03\x06\x06\x1d\n\
    \x0c\n\x05\x06\0\x02\x01\x02\x12\x03\x06\x1f6\n\x0c\n\x05\x06\0\x02\x01\
    \x03\x12\x03\x06AU\n\n\n\x02\x04\0\x12\x04\t\0\x0b\x01\n\n\n\x03\x04\0\
    \x01\x12\x03\t\x08\x14\n\x0b\n\x04\x04\0\x02\0\x12\x03\n\x02\x1f\n\x0c\n\
    \x05\x04\0\x02\0\x04\x12\x03\n\x02\n\n\x0c\n\x05\x04\0\x02\0\x05\x12\x03\
    \n\x0b\x11\n\x0c\n\x05\x04\0\x02\0\x01\x12\x03\n\x12\x1a\n\x0c\n\x05\x04\
    \0\x02\0\x03\x12\x03\n\x1d\x1e\n\n\n\x02\x04\x01\x12\x04\r\0\x0f\x01\n\n\
    \n\x03\x04\x01\x01\x12\x03\r\x08\x15\n\x0b\n\x04\x04\x01\x02\0\x12\x03\
    \x0e\x02\x1c\n\x0c\n\x05\x04\x01\x02\0\x04\x12\x03\x0e\x02\n\n\x0c\n\x05\
    \x04\x01\x02\0\x05\x12\x03\x0e\x0b\x11\n\x0c\n\x05\x04\x01\x02\0\x01\x12\
    \x03\x0e\x12\x17\n\x0c\n\x05\x04\x01\x02\0\x03\x12\x03\x0e\x1a\x1b\n\n\n\
    \x02\x04\x02\x12\x04\x11\0\x13\x01\n\n\n\x03\x04\x02\x01\x12\x03\x11\x08\
    \x1f\n\x0b\n\x04\x04\x02\x02\0\x12\x03\x12\x02\x1d\n\x0c\n\x05\x04\x02\
    \x02\0\x04\x12\x03\x12\x02\n\n\x0c\n\x05\x04\x02\x02\0\x05\x12\x03\x12\
    \x0b\x11\n\x0c\n\x05\x04\x02\x02\0\x01\x12\x03\x12\x12\x18\n\x0c\n\x05\
    \x04\x02\x02\0\x03\x12\x03\x12\x1b\x1c\n\n\n\x02\x04\x03\x12\x04\x15\0\
    \x19\x01\n\n\n\x03\x04\x03\x01\x12\x03\x15\x08\x1c\n\x0b\n\x04\x04\x03\
    \x02\0\x12\x03\x16\x02\x1c\n\x0c\n\x05\x04\x03\x02\0\x04\x12\x03\x16\x02\
    \n\n\x0c\n\x05\x04\x03\x02\0\x05\x12\x03\x16\x0b\x11\n\x0c\n\x05\x04\x03\
    \x02\0\x01\x12\x03\x16\x12\x17\n\x0c\n\x05\x04\x03\x02\0\x03\x12\x03\x16\
    \x1a\x1b\n\x0b\n\x04\x04\x03\x02\x01\x12\x03\x17\x02\x1d\n\x0c\n\x05\x04\
    \x03\x02\x01\x04\x12\x03\x17\x02\n\n\x0c\n\x05\x04\x03\x02\x01\x05\x12\
    \x03\x17\x0b\x11\n\x0c\n\x05\x04\x03\x02\x01\x01\x12\x03\x17\x12\x18\n\
    \x0c\n\x05\x04\x03\x02\x01\x03\x12\x03\x17\x1b\x1c\n\x0b\n\x04\x04\x03\
    \x02\x02\x12\x03\x18\x02&\n\x0c\n\x05\x04\x03\x02\x02\x04\x12\x03\x18\
    \x02\n\n\x0c\n\x05\x04\x03\x02\x02\x05\x12\x03\x18\x0b\x11\n\x0c\n\x05\
    \x04\x03\x02\x02\x01\x12\x03\x18\x12!\n\x0c\n\x05\x04\x03\x02\x02\x03\
    \x12\x03\x18$%\
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
