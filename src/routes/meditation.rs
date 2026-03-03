use crate::db::queries;
use crate::error::AppError;
use crate::models::{
    CompleteMeditationRequest, CompleteMeditationResponse, JournalPromptResponse,
    JournalSubmitRequest, PostMeditationOption, ReflectAnswerRequest, ReflectResponse,
    StartMeditationResponse, UserProgressionInfo,
};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use rand::Rng;

fn get_user_id(jar: &CookieJar) -> Option<String> {
    jar.get("anky_user_id").map(|c| c.value().to_string())
}

fn is_logged_in(jar: &CookieJar) -> bool {
    jar.get("anky_session").is_some()
}

// ===== Reflect Questions Pool =====
const REFLECT_QUESTIONS: &[(&str, &[&str; 4])] = &[
    (
        "What feels most present for you right now?",
        &[
            "A sense of calm",
            "Restlessness or tension",
            "Curiosity about what just happened",
            "Nothing particular — just stillness",
        ],
    ),
    (
        "When you closed your eyes, where did your mind go first?",
        &[
            "To something I need to do",
            "To a memory",
            "Nowhere — it was quiet",
            "To my body and breath",
        ],
    ),
    (
        "What surprised you about sitting still?",
        &[
            "How fast it went",
            "How slow it felt",
            "How noisy my mind is",
            "How peaceful it was",
        ],
    ),
    (
        "If your stillness had a color, what would it be?",
        &[
            "Deep blue",
            "Warm gold",
            "Dark, like space",
            "Shifting — hard to pin down",
        ],
    ),
    (
        "What part of your body held the most tension?",
        &["My shoulders", "My jaw", "My chest", "I didn't notice any"],
    ),
    (
        "How would you describe the quality of silence you found?",
        &[
            "Thick and heavy",
            "Light and spacious",
            "Restless — not really silent",
            "Warm and welcoming",
        ],
    ),
    (
        "What is one word for how you feel right now?",
        &["Grounded", "Open", "Scattered", "Tender"],
    ),
    (
        "Did you resist the stillness or welcome it?",
        &[
            "Resisted — it was uncomfortable",
            "Welcomed it fully",
            "Started resisting, then softened",
            "I'm not sure",
        ],
    ),
    (
        "What would you tell someone who's never sat in silence?",
        &[
            "It's harder than it sounds",
            "It's simpler than you think",
            "Your mind will be loud — that's okay",
            "You might meet yourself",
        ],
    ),
    (
        "If this moment were a landscape, what would it look like?",
        &[
            "An open field",
            "A dark forest",
            "An ocean at dawn",
            "A quiet room",
        ],
    ),
    (
        "What are you avoiding thinking about?",
        &[
            "Something I need to face",
            "Nothing — I feel clear",
            "I'm not sure yet",
            "Everything and nothing",
        ],
    ),
    (
        "How does your breathing feel right now?",
        &[
            "Deep and slow",
            "Shallow and quick",
            "Uneven",
            "I wasn't paying attention to it",
        ],
    ),
    (
        "What emotion is sitting just beneath the surface?",
        &["Sadness", "Joy", "Anxiety", "Something I can't name"],
    ),
    (
        "When was the last time you were truly still?",
        &[
            "Just now",
            "I can't remember",
            "Earlier today",
            "It's been weeks",
        ],
    ),
    (
        "What did the silence ask of you?",
        &[
            "To let go",
            "To pay attention",
            "To be patient",
            "Nothing — it just held me",
        ],
    ),
    (
        "How present were you during the meditation?",
        &[
            "Fully present",
            "Drifted in and out",
            "Mostly in my thoughts",
            "More present than expected",
        ],
    ),
    (
        "What would you like to carry from this moment into your day?",
        &[
            "This stillness",
            "The permission to pause",
            "Awareness of my breath",
            "Nothing specific — just the feeling",
        ],
    ),
    (
        "If you could ask your deeper self one question, what would it be?",
        &[
            "What am I afraid of?",
            "What do I truly want?",
            "What should I let go of?",
            "Am I on the right path?",
        ],
    ),
    (
        "How does your body feel compared to before you sat?",
        &["Lighter", "Heavier", "About the same", "More alive"],
    ),
    (
        "What is the kindest thing you could do for yourself today?",
        &[
            "Rest",
            "Create something",
            "Have an honest conversation",
            "Simply breathe",
        ],
    ),
    (
        "Did any images or memories appear while you sat?",
        &[
            "Yes — vivid ones",
            "Fleeting fragments",
            "No — just darkness",
            "Colors and shapes",
        ],
    ),
    (
        "What is the relationship between you and your thoughts right now?",
        &[
            "I'm watching them pass",
            "They're pulling me around",
            "We're at peace",
            "I barely noticed them",
        ],
    ),
    (
        "How do you feel about doing nothing?",
        &[
            "Guilty — I should be productive",
            "Relieved",
            "Uncomfortable but learning",
            "It felt like the most important thing",
        ],
    ),
    (
        "What pattern in your life would you like to break?",
        &["Rushing", "Avoidance", "Self-criticism", "Distraction"],
    ),
    (
        "If stillness had a taste, what would it be?",
        &[
            "Water — clean and simple",
            "Honey — sweet and thick",
            "Something bitter, like tea",
            "No taste — just space",
        ],
    ),
];

// ===== Journal Prompts Pool =====
const JOURNAL_PROMPTS: &[&str] = &[
    "Write about what silence sounds like to you.",
    "Describe the feeling in your body right now, without judging it.",
    "What is something you've been carrying that you'd like to set down?",
    "Write to the version of yourself from one year ago.",
    "What does your inner critic say most often? Write back to it.",
    "Describe a moment today when you were fully present.",
    "What are you grateful for that you rarely acknowledge?",
    "Write about a fear that no longer serves you.",
    "What would you create if no one would ever see it?",
    "Describe the space between your thoughts.",
    "Write about what rest means to you — not sleep, but true rest.",
    "What emotion have you been avoiding? Write toward it.",
    "Describe your relationship with time right now.",
    "What question keeps returning to you, no matter how many times you answer it?",
    "Write about something you lost that taught you something.",
    "What does your body know that your mind hasn't caught up to yet?",
    "Describe the kind of person you're becoming.",
    "Write about a conversation you need to have but haven't yet.",
    "What is the smallest thing that brings you the most peace?",
    "Write about what it means to be still in a world that never stops.",
    "What part of yourself have you been hiding? Why?",
    "Describe what freedom feels like in your body.",
    "Write about a moment when you surprised yourself.",
    "What would you do with one perfectly empty day?",
    "Write about the difference between being alone and being lonely.",
];

pub async fn start_meditation(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<StartMeditationResponse>, AppError> {
    let user_id =
        get_user_id(&jar).ok_or_else(|| AppError::BadRequest("no user session".into()))?;

    let db = state.db.lock().await;

    let prog = queries::get_or_create_progression(&db, &user_id)?;
    let duration = queries::meditation_duration_for_level(prog.current_meditation_level);

    let session_id = format!("med_{}", uuid::Uuid::new_v4());
    queries::insert_meditation_session(&db, &session_id, &user_id, duration)?;

    Ok(Json(StartMeditationResponse {
        session_id,
        duration,
        level: prog.current_meditation_level,
    }))
}

pub async fn complete_meditation(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<CompleteMeditationRequest>,
) -> Result<Json<CompleteMeditationResponse>, AppError> {
    let user_id =
        get_user_id(&jar).ok_or_else(|| AppError::BadRequest("no user session".into()))?;

    let db = state.db.lock().await;
    let completed =
        queries::complete_meditation_session(&db, &req.session_id, req.duration_actual)?;

    if !completed {
        return Err(AppError::BadRequest(
            "session not found or already completed".into(),
        ));
    }

    let prog = queries::increment_meditation(&db, &user_id)?;

    let options = vec![
        PostMeditationOption {
            id: "reflect".into(),
            label: "reflect".into(),
            locked: false,
            unlock_at: None,
        },
        PostMeditationOption {
            id: "journal".into(),
            label: "journal".into(),
            locked: false,
            unlock_at: None,
        },
        PostMeditationOption {
            id: "write".into(),
            label: "write".into(),
            locked: false,
            unlock_at: None,
        },
        PostMeditationOption {
            id: "sit_again".into(),
            label: "sit again".into(),
            locked: false,
            unlock_at: None,
        },
    ];

    Ok(Json(CompleteMeditationResponse {
        completed: true,
        options,
        level: prog.current_meditation_level,
        total_completed: prog.total_completed,
        streak: prog.current_streak,
    }))
}

pub async fn get_progression(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<UserProgressionInfo>, AppError> {
    let user_id =
        get_user_id(&jar).ok_or_else(|| AppError::BadRequest("no user session".into()))?;

    let db = state.db.lock().await;
    let prog = queries::get_or_create_progression(&db, &user_id)?;
    let duration = queries::meditation_duration_for_level(prog.current_meditation_level);
    let next_at = queries::next_level_threshold(prog.current_meditation_level);

    Ok(Json(UserProgressionInfo {
        level: prog.current_meditation_level,
        duration,
        total_meditations: prog.total_meditations,
        total_completed: prog.total_completed,
        write_unlocked: prog.write_unlocked,
        current_streak: prog.current_streak,
        longest_streak: prog.longest_streak,
        next_level_at: next_at,
    }))
}

pub async fn get_reflect_question(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<ReflectResponse>, AppError> {
    let user_id =
        get_user_id(&jar).ok_or_else(|| AppError::BadRequest("no user session".into()))?;

    let idx = {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..REFLECT_QUESTIONS.len())
    };
    let (question, answers) = &REFLECT_QUESTIONS[idx];

    let interaction_id = format!("int_{}", uuid::Uuid::new_v4());
    let answers_json = serde_json::to_string(&answers.to_vec()).unwrap_or_default();
    let question_str = question.to_string();
    let answers_vec: Vec<String> = answers.iter().map(|s| s.to_string()).collect();

    let db = state.db.lock().await;
    queries::insert_user_interaction(
        &db,
        &interaction_id,
        &user_id,
        None,
        "reflect",
        Some(question),
        Some(&answers_json),
    )?;

    Ok(Json(ReflectResponse {
        interaction_id,
        question: question_str,
        answers: answers_vec,
    }))
}

pub async fn submit_reflect_answer(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<ReflectAnswerRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _user_id =
        get_user_id(&jar).ok_or_else(|| AppError::BadRequest("no user session".into()))?;

    let db = state.db.lock().await;
    let answer_text = format!("answer_{}", req.answer_index);
    queries::update_interaction_response(&db, &req.interaction_id, &answer_text)?;

    let responses = [
        "thank you for noticing that.",
        "that awareness matters more than you think.",
        "anky sees you.",
        "stillness remembers what you tell it.",
        "carry that with you.",
    ];
    let idx = {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..responses.len())
    };
    let response = responses[idx];

    Ok(Json(serde_json::json!({
        "ok": true,
        "message": response,
    })))
}

pub async fn get_journal_prompt(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<JournalPromptResponse>, AppError> {
    let user_id =
        get_user_id(&jar).ok_or_else(|| AppError::BadRequest("no user session".into()))?;

    let idx = {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..JOURNAL_PROMPTS.len())
    };
    let prompt = JOURNAL_PROMPTS[idx];

    let interaction_id = format!("int_{}", uuid::Uuid::new_v4());
    let prompt_str = prompt.to_string();

    let db = state.db.lock().await;
    queries::insert_user_interaction(
        &db,
        &interaction_id,
        &user_id,
        None,
        "journal",
        Some(prompt),
        None,
    )?;

    Ok(Json(JournalPromptResponse {
        interaction_id,
        prompt: prompt_str,
    }))
}

pub async fn submit_journal_entry(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<JournalSubmitRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _user_id =
        get_user_id(&jar).ok_or_else(|| AppError::BadRequest("no user session".into()))?;

    if req.entry.trim().is_empty() {
        return Err(AppError::BadRequest("entry is empty".into()));
    }

    let db = state.db.lock().await;
    queries::update_interaction_response(&db, &req.interaction_id, &req.entry)?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "saved. your words are safe here.",
    })))
}
