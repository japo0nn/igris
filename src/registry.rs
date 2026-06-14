use crate::{
    core::CoreContext,
    error::IgrisError,
    models::metadata::{ModuleMetadata, ModuleType},
    skills::{SkillModule, gui_skill::GuiSkill, memory_skill::MemorySkill, shell_executor::ShellExecutor, user_profile_skill::UserProfileSkill, voice_skill::VoiceSkill, web_search_skill::WebSearchSkill},
};

pub fn init_modules_metadata(context: &CoreContext, secrets: &crate::configs::llm::SecretsConfig) -> Result<Vec<Box<dyn SkillModule>>, IgrisError> {
    let mut skills: Vec<Box<dyn SkillModule>> = Vec::new();

    skills.push(Box::new(ShellExecutor::new()) as Box<dyn SkillModule>);
    skills.push(Box::new(MemorySkill::new(context.clone())) as Box<dyn SkillModule>);
    skills.push(Box::new(GuiSkill::new(context.config.llm.clone())) as Box<dyn SkillModule>);
    skills.push(Box::new(WebSearchSkill::new()) as Box<dyn SkillModule>);
    skills.push(Box::new(UserProfileSkill::new()) as Box<dyn SkillModule>);

    let groq_key = secrets.voice.as_ref().map(|v| v.groq_api_key.clone());
    skills.push(Box::new(VoiceSkill::new(groq_key)) as Box<dyn SkillModule>);

    Ok(skills)
}
