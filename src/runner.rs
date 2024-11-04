use std::{collections::HashMap, io::{Read, Write}};

use anyhow::Result;




pub fn shell_command(command_str: &str) -> std::process::Command {
	let mut command =  std::process::Command::new("bash");
	command.arg("-c")
		.arg(command_str)
		.stdin(std::process::Stdio::piped())
		.stdout(std::process::Stdio::piped());

	return command;
}

pub fn await_command_output(mut cmd: std::process::Command) -> Result<String> {
	let mut output = String::new();

	let mut child = cmd.spawn()?;
	let mut child_output = child.stdout.take()
		.expect("LOGIC ERROR: child does not have stdout");

	let _exit_status = child.wait()?;


	child_output.read_to_string(&mut output)?;
	if output.len() == 0 { return Ok(output); }
	if output.chars().nth(output.len()-1).unwrap() == '\n' {
		unsafe { // I would love to do this the safe way but, it don't see how
			let bytes: *mut [u8] = std::mem::transmute(output.as_str());
			bytes.as_mut_unchecked()[output.len()-1] = b' ';
		}
	} 
	return Ok(output);
}


#[derive(Debug, Clone)]
pub struct BuildInstruction {
	pub command_string: String
}

impl BuildInstruction {
	pub fn new(source: &str, globals: &HashMap<String, String>) -> Self {
		// find any '%'s that have not bee escaped with a '\'
		// and replace the following space-separated token with
		// the global matching its name or "" if that key does not exist
		let mut prev_char = ' ';

		let mut formatted_instruction = String::new();
		let mut token_start: Option<usize> = None;

		for (index, chr) in source.chars().enumerate() {
			match chr {
				'%' => { if prev_char != '\\' {
					token_start = Some(index+1);
				} else { formatted_instruction.push(chr); }},
				' ' | '\n' => {
					if let Some(start) = token_start {
						let var_name = unsafe { source.get_unchecked(start..index) };
						let var_value = match globals.get(var_name) {
							Some(value) => value,
							None => "",
						};
						formatted_instruction.push_str(var_value);
						token_start = None;
					}else {
						formatted_instruction.push(chr);
					}
				}
				_ => { if token_start.is_none() {
					formatted_instruction.push(chr);
				} }
			}
			prev_char = chr;
		}

		return Self { command_string: formatted_instruction };
	}

	pub fn execute(&self) -> Result<()> {
		let _command_output = await_command_output(
			shell_command(self.command_string.as_str())
		)?;
		return Ok(());
	}
}




