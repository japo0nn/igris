use crate::{
    core::CoreContext,
    error::IgrisError,
    models::metadata::{ModuleMetadata, ModuleType},
    skills::{SkillModule, gui_skill::GuiSkill, memory_skill::MemorySkill, shell_executor::ShellExecutor},
};

pub fn init_modules_metadata(
    context: &CoreContext,
) -> Result<Vec<Box<dyn SkillModule>>, IgrisError> {
    let mut modules: Vec<ModuleMetadata> = Vec::new();

    add_or_update_module(
        &mut modules,
        ModuleMetadata {
            name: String::from("ShellExecutor"),
            version: String::from("v0.1.0"),
            _type: ModuleType::Persistent,
            description: String::from("Using shell executor you can run any shell commands"),
            author: Some(String::from("Claude")),
        },
    )?;

    add_or_update_module(
        &mut modules,
        ModuleMetadata {
            name: String::from("Memory"),
            version: String::from("v0.1.0"),
            _type: ModuleType::Ephemeral,
            description: String::from(
                "Memory Skill is connected to IGRIS database where stored user's and assistant all messages",
            ),
            author: None,
        },
    )?;

    add_or_update_module(
        &mut modules,
        ModuleMetadata {
            name: String::from("GuiSkill"),
            version: String::from("v0.1.0"),
            _type: ModuleType::Persistent,
            description: String::from("GUI automation: control mouse, keyboard, take screenshots and open URLs. Cross-platform."),
            author: None,
        },
    )?;

    let mut skills: Vec<Box<dyn SkillModule>> = Vec::new();

    let memory_metadata = find_module(&mut modules, &String::from("Memory"))?;
    skills.push(Box::new(MemorySkill {
        metadata: memory_metadata.clone(),
        context: context.clone(),
    }));

    let shell_metadata = find_module(&mut modules, &String::from("ShellExecutor"))?;
    skills.push(Box::new(ShellExecutor {
        metadata: shell_metadata.clone(),
    }));

    let _gui_metadata = find_module(&mut modules, &String::from("GuiSkill"))?;
    skills.push(Box::new(GuiSkill {
        metadata: _gui_metadata.clone(),
        llm_config: context.config.llm.clone(),
    }));

    return Ok(skills);
}

pub fn find_module<'a>(
    modules: &'a mut Vec<ModuleMetadata>,
    name: &String,
) -> Result<&'a mut ModuleMetadata, IgrisError> {
    let module = modules.iter_mut().find(|x| &x.name == name);

    match module {
        Some(value) => {
            return Ok(value);
        }
        None => {
            return Err(IgrisError::SkillNotFound(format!(
                "Skill not found: {}",
                name
            )));
        }
    }
}

pub fn add_or_update_module(
    modules: &mut Vec<ModuleMetadata>,
    new_module: ModuleMetadata,
) -> Result<(), IgrisError> {
    let search = find_module(modules, &new_module.name);

    match search {
        Ok(module) => {
            if module._type == new_module._type && module.author == new_module.author {
                if module.version == new_module.version {
                    return Err(IgrisError::SkillError(String::from(
                        "Such module already exists\n",
                    )));
                } else {
                    *module = new_module;
                }
            }
        }
        Err(error) => match error {
            IgrisError::SkillNotFound(_) => {
                modules.push(new_module);
            }
            error => {
                return Err(error);
            }
        },
    }

    return Ok(());
}
