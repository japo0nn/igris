use crate::{
    core::CoreContext,
    error::IgrisError,
    skills::{
        SkillModule, gui_skill::GuiSkill, memory_skill::MemorySkill, shell_executor::ShellExecutor,
        user_profile_skill::UserProfileSkill, voice_skill::VoiceSkill,
        web_search_skill::WebSearchSkill,
    },
};

pub fn init_modules_metadata(
    context: &CoreContext,
) -> Result<Vec<Box<dyn SkillModule>>, IgrisError> {
    let mut skills: Vec<Box<dyn SkillModule>> = Vec::new();

    skills.push(Box::new(ShellExecutor::new()) as Box<dyn SkillModule>);
    skills.push(Box::new(MemorySkill::new(context.clone())) as Box<dyn SkillModule>);
    skills.push(Box::new(GuiSkill::new(context.config.llm.clone())) as Box<dyn SkillModule>);
    skills.push(Box::new(WebSearchSkill::new()) as Box<dyn SkillModule>);
    skills.push(Box::new(UserProfileSkill::new()) as Box<dyn SkillModule>);

    skills.push(Box::new(VoiceSkill::new()) as Box<dyn SkillModule>);

    Ok(skills)
}
