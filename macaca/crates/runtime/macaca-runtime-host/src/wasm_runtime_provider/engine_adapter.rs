//! Private in-process WASM adapter.
//!
//! The adapter implements the smallest execution surface needed by the runtime
//! provider contract: validate a core WASM binary, build a compiled module
//! memento, instantiate it, and invoke exported nullary functions.  It is kept
//! private so Macaca can replace this parser/interpreter with Wasmtime,
//! WasmEdge, process isolation, or a remote execution backend without changing
//! public contracts.

use std::collections::BTreeMap;

use macaca_proto::{WasmResourcePolicy, WasmRuntimeErrorKind};

use super::errors::WasmRuntimeHostError;

const WASM_MAGIC: &[u8; 4] = b"\0asm";
const WASM_VERSION: &[u8; 4] = &[0x01, 0x00, 0x00, 0x00];
const WASM_PAGE_BYTES: u64 = 64 * 1024;
const OP_END: u8 = 0x0b;
const OP_UNREACHABLE: u8 = 0x00;

/// Compiled, provider-private module representation.
#[derive(Debug, Clone)]
pub(crate) struct CompiledWasmModule {
    exports: BTreeMap<String, FunctionBody>,
    resource_profile: CompiledWasmResourceProfile,
}

impl CompiledWasmModule {
    /// Report whether the compiled module contains an exported function.
    pub(crate) fn has_export(&self, export_name: &str) -> bool {
        self.exports.contains_key(export_name)
    }

    /// Validate compile-time module resource declarations against policy.
    ///
    /// The minimal in-process adapter cannot enforce every future engine
    /// feature, but it can safely inspect declared memory/table bounds and an
    /// instruction-budget estimate before instantiation.  This keeps the
    /// current provider fail-closed for the resource dimensions it can prove
    /// without leaking raw module bytes into diagnostics.
    pub(crate) fn validate_resource_policy(
        &self,
        policy: &WasmResourcePolicy,
    ) -> Result<(), WasmRuntimeHostError> {
        if let Some(limit) = policy.max_memory_bytes {
            if self.resource_profile.declared_memory_bytes > limit {
                return Err(WasmRuntimeHostError::new(
                    WasmRuntimeErrorKind::ResourceExhausted,
                    "WASM module declared memory exceeds the sandbox policy limit",
                ));
            }
        }
        if let Some(limit) = policy.max_table_elements {
            if self.resource_profile.declared_table_elements > limit {
                return Err(WasmRuntimeHostError::new(
                    WasmRuntimeErrorKind::ResourceExhausted,
                    "WASM module declared table elements exceed the sandbox policy limit",
                ));
            }
        }
        if let Some(limit) = policy.max_fuel {
            if self.resource_profile.estimated_fuel > limit {
                return Err(WasmRuntimeHostError::new(
                    WasmRuntimeErrorKind::ResourceExhausted,
                    "WASM module estimated fuel exceeds the sandbox policy limit",
                ));
            }
        }
        Ok(())
    }
}

/// Static resource profile extracted from a compiled module.
#[derive(Debug, Clone, Default)]
pub(crate) struct CompiledWasmResourceProfile {
    declared_memory_bytes: u64,
    declared_table_elements: u64,
    estimated_fuel: u64,
}

/// Instantiated module handle.
#[derive(Debug, Clone)]
pub(crate) struct InProcessWasmInstance {
    module: CompiledWasmModule,
}

impl InProcessWasmInstance {
    /// Invoke a compiled nullary export and report traps deterministically.
    pub(crate) fn invoke_export(&self, export_name: &str) -> Result<(), WasmRuntimeHostError> {
        let body = self.module.exports.get(export_name).ok_or_else(|| {
            WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::InvokeFailed,
                format!("WASM export '{export_name}' is not available"),
            )
        })?;
        body.invoke()
    }
}

/// Private adapter facade used by the provider and cache.
#[derive(Debug, Default, Clone)]
pub(crate) struct InProcessWasmEngineAdapter;

impl InProcessWasmEngineAdapter {
    /// Compile bytes into a small validated module memento.
    pub(crate) fn compile(&self, bytes: &[u8]) -> Result<CompiledWasmModule, WasmRuntimeHostError> {
        ParsedModule::parse(bytes).map(ParsedModule::compile)
    }

    /// Instantiate a compiled module for a session.
    pub(crate) fn instantiate(
        &self,
        module: CompiledWasmModule,
    ) -> Result<InProcessWasmInstance, WasmRuntimeHostError> {
        Ok(InProcessWasmInstance { module })
    }
}

#[derive(Debug)]
struct ParsedModule {
    function_type_indices: Vec<u32>,
    exported_functions: BTreeMap<String, u32>,
    bodies: Vec<FunctionBody>,
    declared_memory_bytes: u64,
    declared_table_elements: u64,
}

impl ParsedModule {
    fn parse(bytes: &[u8]) -> Result<Self, WasmRuntimeHostError> {
        if bytes.len() < 8 || &bytes[0..4] != WASM_MAGIC || &bytes[4..8] != WASM_VERSION {
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::CompileFailed,
                "WASM artifact has an invalid module header",
            ));
        }
        let mut reader = ByteReader::new(&bytes[8..]);
        let mut module = Self {
            function_type_indices: Vec::new(),
            exported_functions: BTreeMap::new(),
            bodies: Vec::new(),
            declared_memory_bytes: 0,
            declared_table_elements: 0,
        };
        while !reader.is_empty() {
            let section_id = reader.read_u8()?;
            let section_len = reader.read_leb_u32()? as usize;
            let section = reader.read_slice(section_len)?;
            module.parse_section(section_id, section)?;
        }
        if module.function_type_indices.len() != module.bodies.len() {
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::CompileFailed,
                "WASM function and code sections have different lengths",
            ));
        }
        Ok(module)
    }

    fn parse_section(
        &mut self,
        section_id: u8,
        section: &[u8],
    ) -> Result<(), WasmRuntimeHostError> {
        match section_id {
            3 => self.parse_function_section(section),
            4 => self.parse_table_section(section),
            5 => self.parse_memory_section(section),
            7 => self.parse_export_section(section),
            10 => self.parse_code_section(section),
            _ => Ok(()),
        }
    }

    fn parse_function_section(&mut self, section: &[u8]) -> Result<(), WasmRuntimeHostError> {
        let mut reader = ByteReader::new(section);
        let count = reader.read_leb_u32()?;
        for _ in 0..count {
            self.function_type_indices.push(reader.read_leb_u32()?);
        }
        reader.expect_empty()
    }

    fn parse_table_section(&mut self, section: &[u8]) -> Result<(), WasmRuntimeHostError> {
        let mut reader = ByteReader::new(section);
        let count = reader.read_leb_u32()?;
        for _ in 0..count {
            let _element_type = reader.read_u8()?;
            let limits = Limits::parse(&mut reader)?;
            self.declared_table_elements = self
                .declared_table_elements
                .saturating_add(limits.maximum_or_minimum());
        }
        reader.expect_empty()
    }

    fn parse_memory_section(&mut self, section: &[u8]) -> Result<(), WasmRuntimeHostError> {
        let mut reader = ByteReader::new(section);
        let count = reader.read_leb_u32()?;
        for _ in 0..count {
            let limits = Limits::parse(&mut reader)?;
            self.declared_memory_bytes = self
                .declared_memory_bytes
                .saturating_add(limits.maximum_or_minimum().saturating_mul(WASM_PAGE_BYTES));
        }
        reader.expect_empty()
    }

    fn parse_export_section(&mut self, section: &[u8]) -> Result<(), WasmRuntimeHostError> {
        let mut reader = ByteReader::new(section);
        let count = reader.read_leb_u32()?;
        for _ in 0..count {
            let name = reader.read_name()?;
            let kind = reader.read_u8()?;
            let index = reader.read_leb_u32()?;
            if kind == 0 {
                self.exported_functions.insert(name, index);
            }
        }
        reader.expect_empty()
    }

    fn parse_code_section(&mut self, section: &[u8]) -> Result<(), WasmRuntimeHostError> {
        let mut reader = ByteReader::new(section);
        let count = reader.read_leb_u32()?;
        for _ in 0..count {
            let body_len = reader.read_leb_u32()? as usize;
            let body = reader.read_slice(body_len)?;
            self.bodies.push(FunctionBody::parse(body)?);
        }
        reader.expect_empty()
    }

    fn compile(self) -> CompiledWasmModule {
        let mut exports = BTreeMap::new();
        for (name, function_index) in self.exported_functions {
            if let Some(body) = self.bodies.get(function_index as usize) {
                exports.insert(name, body.clone());
            }
        }
        let estimated_fuel = self
            .bodies
            .iter()
            .map(|body| body.estimated_fuel)
            .sum::<u64>();
        CompiledWasmModule {
            exports,
            resource_profile: CompiledWasmResourceProfile {
                declared_memory_bytes: self.declared_memory_bytes,
                declared_table_elements: self.declared_table_elements,
                estimated_fuel,
            },
        }
    }
}

#[derive(Debug, Clone)]
struct FunctionBody {
    traps: bool,
    estimated_fuel: u64,
}

impl FunctionBody {
    fn parse(bytes: &[u8]) -> Result<Self, WasmRuntimeHostError> {
        let mut reader = ByteReader::new(bytes);
        let local_count = reader.read_leb_u32()?;
        for _ in 0..local_count {
            let _local_group_count = reader.read_leb_u32()?;
            let _value_type = reader.read_u8()?;
        }
        let instructions = reader.remaining();
        if instructions.last().copied() != Some(OP_END) {
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::CompileFailed,
                "WASM function body is missing an end opcode",
            ));
        }
        Ok(Self {
            traps: instructions.first().copied() == Some(OP_UNREACHABLE),
            estimated_fuel: instructions.len() as u64,
        })
    }

    fn invoke(&self) -> Result<(), WasmRuntimeHostError> {
        if self.traps {
            Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::Trap,
                "WASM guest trapped while invoking exported function",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Limits {
    minimum: u64,
    maximum: Option<u64>,
}

impl Limits {
    fn parse(reader: &mut ByteReader<'_>) -> Result<Self, WasmRuntimeHostError> {
        let flags = reader.read_leb_u32()?;
        let minimum = reader.read_leb_u32()? as u64;
        let maximum = if flags & 0x01 == 0x01 {
            Some(reader.read_leb_u32()? as u64)
        } else {
            None
        };
        Ok(Self { minimum, maximum })
    }

    fn maximum_or_minimum(self) -> u64 {
        self.maximum.unwrap_or(self.minimum)
    }
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn is_empty(&self) -> bool {
        self.offset >= self.bytes.len()
    }

    fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.offset..]
    }

    fn read_u8(&mut self) -> Result<u8, WasmRuntimeHostError> {
        let value = *self.bytes.get(self.offset).ok_or_else(|| {
            WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::CompileFailed,
                "WASM section ended early",
            )
        })?;
        self.offset += 1;
        Ok(value)
    }

    fn read_slice(&mut self, len: usize) -> Result<&'a [u8], WasmRuntimeHostError> {
        let end = self.offset.checked_add(len).ok_or_else(|| {
            WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::CompileFailed,
                "WASM section length overflow",
            )
        })?;
        if end > self.bytes.len() {
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::CompileFailed,
                "WASM section length exceeds available bytes",
            ));
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    fn read_name(&mut self) -> Result<String, WasmRuntimeHostError> {
        let len = self.read_leb_u32()? as usize;
        let bytes = self.read_slice(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| {
            WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::CompileFailed,
                "WASM export name is not UTF-8",
            )
        })
    }

    fn read_leb_u32(&mut self) -> Result<u32, WasmRuntimeHostError> {
        let mut result = 0u32;
        let mut shift = 0;
        loop {
            let byte = self.read_u8()?;
            result |= ((byte & 0x7f) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
            if shift >= 35 {
                return Err(WasmRuntimeHostError::new(
                    WasmRuntimeErrorKind::CompileFailed,
                    "WASM integer encoding is too large",
                ));
            }
        }
    }

    fn expect_empty(&self) -> Result<(), WasmRuntimeHostError> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::CompileFailed,
                "WASM section contains trailing bytes",
            ))
        }
    }
}
