use crate::{
    core::{CoreContext, llm::generate_topics},
    db::{get_topics, insert_message, insert_topic},
    error::IgrisError,
    memory::Session,
    models::assistant::{
        ActionResponse, Constraints, SystemInfo, TaskObject, TaskObjectSkill, TaskObjectSkillMethod,
        TopicRequest,
    },
    skills::SkillModule,
};

pub fn build_task_object(
    message: &String,
    skills: &Vec<Box<dyn SkillModule>>,
    context: &CoreContext,
    system_response: Option<String>,
) -> Result<TaskObject, IgrisError> {
    #[cfg(target_os = "windows")]
    let shell = std::env::var("COMSPEC").unwrap_or("cmd.exe".to_string());

    #[cfg(not(target_os = "windows"))]
    let shell = std::env::var("SHELL").unwrap_or("sh".to_string());

    let capabilities = vec![
        "execute_module".to_string(),
        "prompt_user".to_string(),
        "permission_request".to_string(),
        "request_data".to_string(),
        "generate_chunk".to_string(),
    ];

    let task_object = TaskObject {
        message: message.clone(),
        system_info: SystemInfo {
            os: std::env::consts::OS.to_string(),
            shell: shell,
        },
        system_response: system_response,
        skills: build_skills_context(skills)?,
        all_topics: get_topics(&context.connection.lock().unwrap())?,
        capabilities,
        constraints: Constraints {
            max_iterations: 10,
            max_fix_iterations: 5,
            max_tokens: context.config.llm.max_tokens,
        },
    };

    return Ok(task_object);
}

fn build_skills_context(
    skills: &Vec<Box<dyn SkillModule>>,
) -> Result<Vec<TaskObjectSkill>, IgrisError> {
    let mut context: Vec<TaskObjectSkill> = Vec::new();

    for skill in skills {
        let skill_metadata = skill.get_metadata();
        let skill_methods = skill.available_methods();
        let mut context_skill_methods: Vec<TaskObjectSkillMethod> = Vec::new();

        for method in skill_methods {
            context_skill_methods.push(TaskObjectSkillMethod {
                method: method.method,
                description: method.description,
                args_description: method.args_description,
            });
        }

        context.push(TaskObjectSkill {
            name: skill_metadata.name.clone(),
            description: skill_metadata.description.clone(),
            available_methods: context_skill_methods,
        });
    }

    return Ok(context);
}

pub async fn spawn_save_message(
    context: &CoreContext,
    role: String,
    message: &ActionResponse,
    session: &Session,
) -> Result<(), IgrisError> {
    save_message_with_topics(context, role, message, session).await
}

async fn save_message_with_topics(
    context: &CoreContext,
    role: String,
    message: &ActionResponse,
    session: &Session,
) -> Result<(), IgrisError> {
    let message_id = insert_message(
        &context.connection.lock().unwrap(),
        role,
        &message,
        &session,
    )?;

    let existing_topics = get_topics(&context.connection.lock().unwrap())?;

    let topic_request = TopicRequest {
        message: message.message.clone(),
        existing_topics: existing_topics,
    };

    let topic_json = serde_json::json!(topic_request).to_string();

    let max_tokens = context.config.topic_llm.max_tokens;
    let content = generate_topics(topic_json, &context.config, max_tokens).await?;

    let generated_topics: Vec<String> = serde_json::from_str(&content).unwrap_or_default();

    insert_topic(
        &context.connection.lock().unwrap(),
        generated_topics,
        message_id,
    )?;

    Ok(())
}
