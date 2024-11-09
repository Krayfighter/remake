#![feature(ascii_char)]
#![feature(ptr_as_ref_unchecked)]

use std::collections::HashMap;

use anyhow::Result;

#[macro_use] extern crate anyhow;

mod runner;



#[derive(Debug, Clone)]
struct BuildTarget {
	name: String,
	build_commands: Vec<runner::BuildInstruction>,
	dependencies: Vec<String>,
}

impl BuildTarget {
	pub fn build(self) {
		for instruction in self.build_commands {
			eprintln!("INFO: executing instruction: {}", instruction.command_string);
			instruction.execute()
				.expect("FATAL: failed to execute build instruction");
		}
	}
}


fn parse_target_name<'a>(source: &'a str, start: usize) -> Result<Option<(&'a str, usize)>> {
	// let mut index = start;
	let mut token_start = start;
	let mut inside_token = false;

	// let mut index = start;
	for (index, chr) in source.chars().skip(start).enumerate() {
		match chr {
			':' => {
				if !inside_token { bail!("Expected TargetName before start of target build definition"); }
				let token_end = start + index;
				return Ok( Some( (
					// this operation should always be sound since start and index must
					// always be within the source string's bounds
					unsafe { source.get_unchecked(token_start..token_end) },
					token_end
				) ) );
			},
			// ' ' => bail!("Expected ':' but found a token seperator ' '"),
			' ' | '\n' | '\t' => {
				if inside_token {
					bail!("Expected ':' but found token separator");
				}
				token_start += 1;
			},
			_ => { inside_token = true; },
		}
	}
	if inside_token {
		bail!("Expected target name to be followed by a ':'");
	}else {
		return Ok(None);
	}
}

fn parse_target_dependents<'a>(source: &'a str, start: usize) -> Result<(Vec<&'a str>, usize)> {
	let mut token_start = start;
	let mut inside_token = false;
	// let mut token_end = 0;

	let mut dependents = Vec::<&str>::new();

	let mut token_end = start;
	for chr in source.chars().skip(start) {
		token_end += 1;
		match chr {
			' ' | '\t' => {
				if inside_token {
					dependents.push(unsafe {source.get_unchecked(token_start..token_end-1) });
					token_start = token_end;
					inside_token = false;
				}else {
					token_start += 1;
				}
			},
			'\n' => {
				if inside_token {
					dependents.push(unsafe {source.get_unchecked(token_start..token_end-1) });
				}
				return Ok((dependents, token_end));
			},
			_ => { inside_token = true; },
		}
	}
	bail!("Unexpected end of character stream");
}

fn parse_build_instructions<'a>(
	source: &'a str,
	start: usize,
	globals: &HashMap<String, String>
) -> Result<(Vec<runner::BuildInstruction>, usize)> {
	let mut token_start = start;
	let mut inside_token = false;
	let mut inside_line = true;

	// let mut instructions =  Vec::<std::process::Command>::new();
	let mut instructions = Vec::<runner::BuildInstruction>::new();

	let mut token_end = start;
	for (index, chr) in source.chars().skip(start).enumerate() {
		token_end += 1;

		// TODO make this algorithm more robust
		match chr {
			'\t' => {
				if !inside_token { token_start += 1; }
			}
			'\n' => {
				if !inside_line { return Ok((instructions, token_end)) }
				else {
					instructions.push( runner::BuildInstruction::new( unsafe{
						source.get_unchecked(token_start..token_end-1)
					}, globals ) );
					inside_line = false;
					inside_token = false;
				}
			}
			_ => {
				if !inside_token { inside_token = true; inside_line = true; token_start = start + index; }
			}
		}
	}

	return Ok((vec!(), start));
}

fn parse_global(
	source: &str,
	start: usize,
	globals: &mut HashMap<String, String>
) -> Result<usize> {
	let mut token_start = start + 7;

	let mut token_end = token_start;
	let mut inside_token = false;

	let mut variable_name: Option<String> = None;
	// let command_string: Option<&str> = None;

	for chr in source.chars().skip(token_start) {
		token_end += 1;
		match chr {
			' ' => {
				if inside_token {
					if variable_name.is_none() {
						variable_name = Some(String::from( unsafe {
							source.get_unchecked(token_start..token_end-1)
						} ));
						token_start = token_end;
					}else { continue; }
					inside_token = false;
				}else {
					token_start += 1;
				}
			},
			'\n' => {
				if variable_name.is_none() {
					if !inside_token { bail!("Error: exports require a name"); }
					variable_name = Some(String::from( unsafe {
						source.get_unchecked(token_start..token_end-1)
					} ));
				}
				let mut command_string = String::from("echo ");
				// let cmd_str = unsafe { source.get_unchecked(token_start..token_end-1) };
				command_string.push_str(unsafe { source.get_unchecked(token_start..token_end-1) });
				let var_value = runner::await_command_output(
					runner::shell_command(command_string.as_str())
				)?;
				globals.insert(variable_name.unwrap(), var_value);
				return Ok(token_end);
			},
			'=' => {
				if inside_token {
					if variable_name.is_none() {
						variable_name = Some(String::from( unsafe {
							source.get_unchecked(token_start..token_end)
						} ));
						token_start = token_end;
					}
				}
				if variable_name.is_none() {
					bail!("Error: exports require a name");
				}
				inside_token = false;
			}
			_ => { inside_token = true; }
		}
	}

	return Ok(token_end);
}

fn parse(source: &str, globals: &mut HashMap<String, String>) -> Result<Vec<BuildTarget>> {
	let mut token_start: usize = 0;
	let mut targets = Vec::<BuildTarget>::new();

	'parse_loop: loop { // loop until there are no more targets to parse
		// if the line starts with "export " then treat it as a global
		// variable definition
		match source.chars().nth(token_start) {
			Some('\n') => { token_start += 1; continue; }
			None => { break 'parse_loop; }
			Some(_) => {},
		}
		
		if source.get(token_start..token_start+7)
			.unwrap_or("") == "export "
		{
			// parse global variable
			token_start = parse_global(source, token_start, globals)?;
			continue 'parse_loop;
		}
		let target_name;
		(target_name, token_start) = match parse_target_name(source, token_start)? {
			Some(mut token) => {
				token.1 += 1; // just past the ':' separator
				token
			},
			None => break 'parse_loop
		};
		let target_dependents;
		(target_dependents, token_start) = parse_target_dependents(source, token_start)?;

		let target_build_instructions;
		(target_build_instructions, token_start) = parse_build_instructions(source, token_start, globals)?;

		targets.push(BuildTarget {
			name: target_name.to_owned(),
			build_commands: target_build_instructions,
			dependencies: target_dependents.into_iter()
				.map(|str| str.to_owned())
				.collect()
		} );
	}

	return Ok(targets);
}

#[derive(Debug)]
struct BuildTree {
	target_map: Vec<(BuildTarget, usize)>,
}

impl BuildTree {
	pub fn new(
		main_target: usize,
		source_targets: &mut Vec<BuildTarget>,
		// _self: Option<&mut Self>,
	) -> Result<Self> {
		let mut _self = Self{ target_map: vec!() };

		_self.add_branch(main_target, source_targets, None)?;

		return Ok(_self);
	}

	pub fn build(self) {
		// this buffer hold 'stages' of targets that can all be executed in
		// parallel each with a depth, and must be executed from shallowest
		// to deepest
		let mut build_stages = Vec::<Vec<BuildTarget>>::new();
		let mut stage_depths = Vec::<usize>::new();

		for (target, depth) in self.target_map.into_iter() {
			if let Some((index, _depth)) = stage_depths.iter()
				.enumerate().find(|(_index, item_depth)| **item_depth == depth)
			{
				build_stages[index].push(target);
			}else {
				build_stages.push(vec!(target));
				stage_depths.push(depth);
			}
		}

		// hand-rolled bubble sort, may not be the best at scale
		// but should work well enough
		loop {
			let mut has_finished = true;
			let mut prev_item: Option<usize> = None;
			for index in 0..build_stages.len() {
				let current = stage_depths[index];
				if let Some(prev) = prev_item {
					if prev > current {
						stage_depths.swap(current, prev);
						build_stages.swap(current, prev);
						has_finished = false;
					}
				}
				let current = stage_depths[index];
				prev_item = Some(current);
			}
			if has_finished { break; }
		}

		for stage in build_stages.into_iter().rev() {
			// TODO make this parallel
			for target in stage {
				target.build();
			}
		}
	}

	// returns the max depth recursed into
	fn add_branch(
		&mut self,
		index: usize,
		source_targets: &mut Vec<BuildTarget>,
		depth: Option<usize>,
	) -> Result<usize> {
		let depth = match depth {
			Some(depth) => depth,
			None => 0,
		};

		// this is a leaf node
		if source_targets[index].dependencies.len() == 0 {
			self.insert_node_depth(index, source_targets, depth);
			return Ok(depth);
		}

		for dep_target_name in source_targets[index].clone().dependencies.iter() {
			if let Some((index, _target_ref)) = source_targets.iter()
				.enumerate()
				.find(|target_ref| target_ref.1.name.as_str() == dep_target_name.as_str()
			) {
				self.add_branch(index, source_targets, Some(depth+1))?;
			}
			else { bail!("Error: dependency not found: {}", dep_target_name); }
		}

		self.insert_node_depth(index, source_targets, depth);
		return Ok(depth);
	}

	// insert this target as a node if it is a higher depth than any other node
	fn insert_node_depth(
		&mut self,
		target_index: usize,
		source_targets: &mut Vec<BuildTarget>,
		depth: usize,
	) {
		let target = source_targets[target_index].clone();

		if let Some((prev_target_index, target_ref)) = self.target_map.iter()
			.enumerate()
			.find(|target_ref| target_ref.1.0.name == target.name)
		{
			if target_ref.1 < depth {
				let _ = self.target_map.remove(prev_target_index);
			}
		}
		self.target_map.push((target, depth));
	}
}



fn main() -> Result<()> {

	let args = std::env::args();

	if args.len() == 1 {
		eprintln!("Error: missing one positional argument {{ target }}");
		bail!("Invalid Invocation");
	}
	let mut main_target: Option<String> = None;
	let mut debug = cfg!(debug_assertions);
	for arg in args.skip(1) {
		if arg.starts_with('-') {
			match arg.as_str() {
				"--debug" => debug = true,
				_ => bail!("FATAL: unrecognized argument: {}", arg)
			}
		}
		else {
			if main_target.is_some() { bail!("FATAL: cannot have 2 main targets"); }
			main_target = Some(arg);
		}
	}

	let main_target = main_target.expect("FATAL: must run with a main target");

	// if args.len() > 2 {
	// 	eprintln!("Error: too many arguments, only one positional {{ target }} argument allowed");
	// 	bail!("Invalid Invocation");
	// }

	// let main_target = args.nth(1)
	// 	.expect("FATAL: unable to fetch command arg");

	let mut globals = std::collections::HashMap::<String, String>::new();
	
	let build_file = std::path::PathBuf::from("RemakeRunner");
	let mut targets = parse(std::fs::read(build_file)
		.expect("FATAL: failed to read from build file")
		.as_ascii()
		.unwrap()
		.as_str(), &mut globals
	)?;

	// TODO do not pipe the stdout of the main target
	if debug {
		eprintln!("DBG: globals {{{:#?}}}", globals);
		eprintln!("DBG: build targets {{{:#?}}}", targets);
	}

	if let Some((target_index, _target_ref)) = targets.iter()
		.enumerate().find(|(_index, target_ref)| target_ref.name == main_target)
	{
		let build_tree = BuildTree::new(
			target_index, &mut targets
		)?;

		if debug { eprintln!("DBG: build tree {{{:#?}}}", build_tree); }

		build_tree.build();
	}else {
		eprintln!("Error: unable to find target in RemakeRunner: {}", main_target);
		bail!("Invalid Invocation");
	}

	return Ok(());
}

