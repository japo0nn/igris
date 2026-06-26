use crate::{
    core::{llm::generate_topics, CoreContext},
    db::{get_topics, insert_message, insert_topic},
    error::IgrisError,
    memory::Session,
    models::assistant::{
        ActionResponse, Constraints, SystemInfo, TaskObject, TaskObjectSkill,
        TaskObjectSkillMethod, TopicRequest,
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
        all_topics: get_topics(&context.connection.lock().unwrap_or_else(|e| e.into_inner()))?,
        capabilities,
        constraints: Constraints {
            max_iterations: 10,
            max_fix_iterations: 5,
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
    save_message_with_topics(context, role, message, None, session).await
}

pub async fn spawn_save_message_with_raw(
    context: &CoreContext,
    role: String,
    message: &ActionResponse,
    raw_json: Option<&str>,
    session: &Session,
) -> Result<(), IgrisError> {
    save_message_with_topics(context, role, message, raw_json, session).await
}

async fn save_message_with_topics(
    context: &CoreContext,
    role: String,
    message: &ActionResponse,
    raw_json: Option<&str>,
    session: &Session,
) -> Result<(), IgrisError> {
    let message_id = insert_message(
        &context.connection.lock().unwrap_or_else(|e| e.into_inner()),
        role,
        &message,
        raw_json,
        &session,
    )?;

    let existing_topics =
        get_topics(&context.connection.lock().unwrap_or_else(|e| e.into_inner()))?;

    let topic_request = TopicRequest {
        message: message.message.clone(),
        existing_topics: existing_topics,
    };

    let topic_json = serde_json::json!(topic_request).to_string();

    let content = generate_topics(topic_json, &context.config).await?;

    let generated_topics: Vec<String> = serde_json::from_str(&content).unwrap_or_default();

    if generated_topics.len() > 0 {
        insert_topic(
            &context.connection.lock().unwrap_or_else(|e| e.into_inner()),
            generated_topics,
            message_id,
        )?;
    }

    Ok(())
}
