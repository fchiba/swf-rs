use avm1::opcode::OpCode;
use avm1::types::*;
use read::SwfRead;
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read, Result};

pub struct Reader<R: Read> {
    inner: R,
    version: u8,
}

impl<R: Read> SwfRead<R> for Reader<R> {
    fn get_version(&self) -> u8 {
        self.version
    }

    fn get_inner(&mut self) -> &mut R {
        &mut self.inner
    }
}

impl<R: Read> Reader<R> {
    pub fn new(inner: R, version: u8) -> Reader<R> {
        Reader {
            inner: inner,
            version: version,
        }
    }

    pub fn read_action_list(&mut self) -> Result<ActionList> {
        let mut actions = Vec::new();
        let mut positions = vec![0];
        let mut position = 0;
        while let Some(action_with_size) = try!(self.read_action()) {
            actions.push(action_with_size.0);
            position += action_with_size.1;
            positions.push(position);
        }
        let position_to_idx: HashMap<usize, usize> = positions
            .iter()
            .enumerate()
            .map(|(idx, pos)| (*pos, idx))
            .collect();
        trace!("potions {:?}", positions);
        trace!("position_to_idx {:?}", position_to_idx);

        for (idx, action) in actions.iter_mut().enumerate() {
            match action {
                Action::If { offset, jump_to } | Action::Jump { offset, jump_to } => {
                    trace!("offset {}", offset);
                    trace!("current_idx {}", idx);
                    let current_pos = positions[idx + 1] as i16;
                    trace!("current_pos {}", current_pos);
                    let next_pos = (current_pos + *offset) as usize;
                    trace!("next_pos {}", next_pos);
                    *jump_to = position_to_idx[&next_pos] as i16;
                    trace!("next_idx {}", position_to_idx[&next_pos]);
                }
                _ => {}
            }
        }

        Ok(actions)
    }

    pub fn read_action(&mut self) -> Result<Option<(Action, usize)>> {
        let result = self.read_opcode_and_length();
        if let Err(err) = result {
            use std::io::ErrorKind::UnexpectedEof;
            if err.kind() == UnexpectedEof {
                return Ok(None);
            } else {
                return Err(err);
            }
        }
        let (opcode, length) = result.unwrap();
        trace!("opcode {} length {}", opcode, length);

        let mut action;
        let mut code_length = 0; // for DefineFunction / DefineFunction2
        {
            let mut action_reader =
                Reader::new(self.inner.by_ref().take(length as u64), self.version);

            use num::FromPrimitive;
            action = if let Some(op) = OpCode::from_u8(opcode) {
                match op {
                    OpCode::End => return Ok(None),

                    OpCode::Add => Action::Add,
                    OpCode::Add2 => Action::Add2,
                    OpCode::And => Action::And,
                    OpCode::AsciiToChar => Action::AsciiToChar,
                    OpCode::BitAnd => Action::BitAnd,
                    OpCode::BitLShift => Action::BitLShift,
                    OpCode::BitOr => Action::BitOr,
                    OpCode::BitRShift => Action::BitRShift,
                    OpCode::BitURShift => Action::BitURShift,
                    OpCode::BitXor => Action::BitXor,
                    OpCode::Call => Action::Call,
                    OpCode::CallFunction => Action::CallFunction,
                    OpCode::CallMethod => Action::CallMethod,
                    OpCode::CastOp => Action::CastOp,
                    OpCode::CharToAscii => Action::CharToAscii,
                    OpCode::CloneSprite => Action::CloneSprite,
                    OpCode::ConstantPool => {
                        let mut constants = vec![];
                        for _ in 0..action_reader.read_u16()? {
                            constants.push(action_reader.read_c_string()?);
                        }
                        Action::ConstantPool(constants)
                    }
                    OpCode::Decrement => Action::Decrement,
                    OpCode::DefineFunction => {
                        let action = action_reader.read_define_function()?;
                        code_length = action_reader.read_u16()?;
                        action
                    }
                    OpCode::DefineFunction2 => {
                        let action = action_reader.read_define_function_2()?;
                        code_length = action_reader.read_u16()?;
                        action
                    }
                    OpCode::DefineLocal => Action::DefineLocal,
                    OpCode::DefineLocal2 => Action::DefineLocal2,
                    OpCode::Delete => Action::Delete,
                    OpCode::Delete2 => Action::Delete2,
                    OpCode::Divide => Action::Divide,
                    OpCode::EndDrag => Action::EndDrag,
                    OpCode::Enumerate => Action::Enumerate,
                    OpCode::Enumerate2 => Action::Enumerate2,
                    OpCode::Equals => Action::Equals,
                    OpCode::Equals2 => Action::Equals2,
                    OpCode::Extends => Action::Extends,
                    OpCode::GetMember => Action::GetMember,
                    OpCode::GetProperty => Action::GetProperty,
                    OpCode::GetTime => Action::GetTime,
                    OpCode::GetUrl => Action::GetUrl {
                        url: try!(action_reader.read_c_string()),
                        target: try!(action_reader.read_c_string()),
                    },
                    OpCode::GetUrl2 => {
                        let flags = try!(action_reader.read_u8());
                        Action::GetUrl2 {
                            is_target_sprite: flags & 0b10 != 0,
                            is_load_vars: flags & 0b1 != 0,
                            send_vars_method: match flags >> 6 {
                                0 => SendVarsMethod::None,
                                1 => SendVarsMethod::Get,
                                2 => SendVarsMethod::Post,
                                _ => {
                                    return Err(Error::new(
                                        ErrorKind::InvalidData,
                                        "Invalid HTTP method in ActionGetUrl2",
                                    ))
                                }
                            },
                        }
                    }
                    OpCode::GetVariable => Action::GetVariable,
                    OpCode::GotoFrame => {
                        let frame = try!(action_reader.read_u16());
                        Action::GotoFrame(frame)
                    }
                    OpCode::GotoFrame2 => {
                        let flags = try!(action_reader.read_u8());
                        Action::GotoFrame2 {
                            set_playing: flags & 0b1 != 0,
                            scene_offset: if flags & 0b10 != 0 {
                                try!(action_reader.read_u16())
                            } else {
                                0
                            },
                        }
                    }
                    OpCode::GotoLabel => Action::GotoLabel(try!(action_reader.read_c_string())),
                    OpCode::Greater => Action::Greater,
                    OpCode::If => Action::If {
                        offset: try!(action_reader.read_i16()),
                        jump_to: 0, // update later
                    },
                    OpCode::ImplementsOp => Action::ImplementsOp,
                    OpCode::Increment => Action::Increment,
                    OpCode::InitArray => Action::InitArray,
                    OpCode::InitObject => Action::InitObject,
                    OpCode::InstanceOf => Action::InstanceOf,
                    OpCode::Jump => Action::Jump {
                        offset: try!(action_reader.read_i16()),
                        jump_to: 0, // update later
                    },
                    OpCode::Less => Action::Less,
                    OpCode::Less2 => Action::Less2,
                    OpCode::MBAsciiToChar => Action::MBAsciiToChar,
                    OpCode::MBCharToAscii => Action::MBCharToAscii,
                    OpCode::MBStringExtract => Action::MBStringExtract,
                    OpCode::MBStringLength => Action::MBStringLength,
                    OpCode::Modulo => Action::Modulo,
                    OpCode::Multiply => Action::Multiply,
                    OpCode::NewMethod => Action::NewMethod,
                    OpCode::NewObject => Action::NewObject,
                    OpCode::NextFrame => Action::NextFrame,
                    OpCode::Not => Action::Not,
                    OpCode::Or => Action::Or,
                    OpCode::Play => Action::Play,
                    OpCode::Pop => Action::Pop,
                    OpCode::PreviousFrame => Action::PreviousFrame,
                    // TODO: Verify correct version for complex types.
                    OpCode::Push => {
                        let mut values = vec![];
                        while let Ok(value) = action_reader.read_push_value() {
                            values.push(value);
                        }
                        Action::Push(values)
                    }
                    OpCode::PushDuplicate => Action::PushDuplicate,
                    OpCode::RandomNumber => Action::RandomNumber,
                    OpCode::RemoveSprite => Action::RemoveSprite,
                    OpCode::Return => Action::Return,
                    OpCode::SetMember => Action::SetMember,
                    OpCode::SetProperty => Action::SetProperty,
                    OpCode::SetTarget => Action::SetTarget(action_reader.read_c_string()?),
                    OpCode::SetTarget2 => Action::SetTarget2,
                    OpCode::SetVariable => Action::SetVariable,
                    OpCode::StackSwap => Action::StackSwap,
                    OpCode::StartDrag => Action::StartDrag,
                    OpCode::Stop => Action::Stop,
                    OpCode::StopSounds => Action::StopSounds,
                    OpCode::StoreRegister => Action::StoreRegister(action_reader.read_u8()?),
                    OpCode::StrictEquals => Action::StrictEquals,
                    OpCode::StringAdd => Action::StringAdd,
                    OpCode::StringEquals => Action::StringEquals,
                    OpCode::StringExtract => Action::StringExtract,
                    OpCode::StringGreater => Action::StringGreater,
                    OpCode::StringLength => Action::StringLength,
                    OpCode::StringLess => Action::StringLess,
                    OpCode::Subtract => Action::Subtract,
                    OpCode::TargetPath => Action::TargetPath,
                    OpCode::Throw => Action::Throw,
                    OpCode::ToggleQuality => Action::ToggleQuality,
                    OpCode::ToInteger => Action::ToInteger,
                    OpCode::ToNumber => Action::ToNumber,
                    OpCode::ToString => Action::ToString,
                    OpCode::Trace => Action::Trace,
                    OpCode::Try => action_reader.read_try()?,
                    OpCode::TypeOf => Action::TypeOf,
                    OpCode::WaitForFrame => Action::WaitForFrame {
                        frame: try!(action_reader.read_u16()),
                        num_actions_to_skip: try!(action_reader.read_u8()),
                    },
                    OpCode::With => {
                        let code_length = action_reader.read_u16()?;
                        let mut with_reader = Reader::new(
                            (&mut action_reader.inner as &mut Read).take(code_length as u64),
                            self.version,
                        );
                        Action::With {
                            actions: with_reader.read_action_list()?,
                        }
                    }
                    OpCode::WaitForFrame2 => Action::WaitForFrame2 {
                        num_actions_to_skip: try!(action_reader.read_u8()),
                    },
                }
            } else {
                action_reader.read_unknown_action(opcode, length)?
            };
        };

        action = match action {
            Action::DefineFunction {
                name,
                params,
                actions: _,
            } => {
                let mut fn_reader = Reader::new(
                    (&mut self.inner as &mut Read).take(code_length as u64),
                    self.version,
                );
                let actions = fn_reader.read_action_list()?;

                Action::DefineFunction {
                    name: name,
                    params: params,
                    actions: actions,
                }
            }
            Action::DefineFunction2(mut function) => {
                let mut fn_reader = Reader::new(
                    (&mut self.inner as &mut Read).take(code_length as u64),
                    self.version,
                );
                let actions = fn_reader.read_action_list()?;
                function.actions = actions;
                Action::DefineFunction2(function)
            }
            _ => action,
        };

        let size = if length == 0 { 1 } else { 1 + 2 + length };
        Ok(Some((action, size)))
    }

    pub fn read_opcode_and_length(&mut self) -> Result<(u8, usize)> {
        let opcode = try!(self.read_u8());
        let length = if opcode >= 0x80 {
            try!(self.read_u16()) as usize
        } else {
            0
        };
        Ok((opcode, length))
    }

    fn read_unknown_action(&mut self, opcode: u8, length: usize) -> Result<Action> {
        let mut data = vec![0u8; length];
        self.inner.read_exact(&mut data)?;
        Ok(Action::Unknown {
            opcode: opcode,
            data: data,
        })
    }

    fn read_push_value(&mut self) -> Result<Value> {
        let value = match try!(self.read_u8()) {
            0 => Value::Str(try!(self.read_c_string())),
            1 => Value::Float(try!(self.read_f32())),
            2 => Value::Null,
            3 => Value::Undefined,
            4 => Value::Register(try!(self.read_u8())),
            5 => Value::Bool(try!(self.read_u8()) != 0),
            6 => Value::Double(try!(self.read_f64())),
            7 => Value::Int(try!(self.read_u32())),
            8 => Value::ConstantPool(try!(self.read_u8()) as u16),
            9 => Value::ConstantPool(try!(self.read_u16())),
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid value type in ActionPush",
                ))
            }
        };
        Ok(value)
    }

    fn read_define_function(&mut self) -> Result<Action> {
        let name = self.read_c_string()?;
        let num_params = self.read_u16()?;
        let mut params = Vec::with_capacity(num_params as usize);
        for _ in 0..num_params {
            params.push(self.read_c_string()?);
        }
        Ok(Action::DefineFunction {
            name: name,
            params: params,
            actions: Vec::new(),
        })
    }

    fn read_define_function_2(&mut self) -> Result<Action> {
        let name = self.read_c_string()?;
        let num_params = self.read_u16()?;
        let num_registers = self.read_u8()?; // Number of registers
        let flags = self.read_u16()?;
        let mut params = Vec::with_capacity(num_params as usize);
        for _ in 0..num_params {
            let register = self.read_u8()?;
            params.push(FunctionParam {
                name: self.read_c_string()?,
                register_index: if register == 0 { None } else { Some(register) },
            });
        }
        Ok(Action::DefineFunction2(Function {
            name: name,
            params: params,
            num_registers: num_registers,
            preload_global: flags & 0b1_00000000 != 0,
            preload_parent: flags & 0b10000000 != 0,
            preload_root: flags & 0b1000000 != 0,
            suppress_super: flags & 0b100000 != 0,
            preload_super: flags & 0b10000 != 0,
            suppress_arguments: flags & 0b1000 != 0,
            preload_arguments: flags & 0b100 != 0,
            suppress_this: flags & 0b10 != 0,
            preload_this: flags & 0b1 != 0,
            actions: Vec::new(),
        }))
    }

    fn read_try(&mut self) -> Result<Action> {
        let flags = self.read_u8()?;
        let try_length = self.read_u16()?;
        let catch_length = self.read_u16()?;
        let finally_length = self.read_u16()?;
        let catch_var = if flags & 0b100 != 0 {
            CatchVar::Var(self.read_c_string()?)
        } else {
            CatchVar::Register(self.read_u8()?)
        };
        let try_actions = {
            let mut fn_reader = Reader::new(
                (&mut self.inner as &mut Read).take(try_length as u64),
                self.version,
            );
            fn_reader.read_action_list()?
        };
        let catch_actions = {
            let mut fn_reader = Reader::new(
                (&mut self.inner as &mut Read).take(catch_length as u64),
                self.version,
            );
            fn_reader.read_action_list()?
        };
        let finally_actions = {
            let mut fn_reader = Reader::new(
                (&mut self.inner as &mut Read).take(finally_length as u64),
                self.version,
            );
            fn_reader.read_action_list()?
        };
        Ok(Action::Try(TryBlock {
            try: try_actions,
            catch: if flags & 0b1 != 0 {
                Some((catch_var, catch_actions))
            } else {
                None
            },
            finally: if flags & 0b10 != 0 {
                Some(finally_actions)
            } else {
                None
            },
        }))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use test_data;

    #[test]
    fn read_action() {
        for (swf_version, expected_action, action_bytes) in test_data::avm1_tests() {
            let mut reader = Reader::new(&action_bytes[..], swf_version);
            let parsed_action = reader.read_action().unwrap().unwrap();
            if parsed_action.0 != expected_action {
                // Failed, result doesn't match.
                panic!(
                    "Incorrectly parsed action.\nRead:\n{:?}\n\nExpected:\n{:?}",
                    parsed_action, expected_action
                );
            }
        }
    }
}
