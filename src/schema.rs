table! {
    artifacts (id) {
        id -> Int4,
        path -> Varchar,
    }
}

table! {
    endpoints (id) {
        id -> Int4,
        name -> Varchar,
    }
}

table! {
    envvars (id) {
        id -> Int4,
        name -> Varchar,
        value -> Varchar,
    }
}

table! {
    githashes (id) {
        id -> Int4,
        hash -> Varchar,
    }
}

table! {
    images (id) {
        id -> Int4,
        name -> Varchar,
    }
}

table! {
    job_envs (id) {
        id -> Int4,
        job_id -> Int4,
        env_id -> Int4,
    }
}

table! {
    job_input_artifacts (id) {
        id -> Int4,
        job_id -> Int4,
        artifact_id -> Int4,
    }
}

table! {
    jobs (id) {
        id -> Int4,
        submit_id -> Int4,
        endpoint_id -> Int4,
        package_id -> Int4,
        image_id -> Int4,
        artifact_id -> Int4,
        container_hash -> Varchar,
        script_text -> Text,
        log_text -> Text,
    }
}

table! {
    packages (id) {
        id -> Int4,
        name -> Varchar,
        version -> Varchar,
    }
}

table! {
    submit_envs (id) {
        id -> Int4,
        submit_id -> Int4,
        env_id -> Int4,
    }
}

table! {
    submits (id) {
        id -> Int4,
        uuid -> Uuid,
        submit_time -> Timestamptz,
        requested_image_id -> Int4,
        requested_package_id -> Int4,
        repo_hash_id -> Int4,
        tree -> Jsonb,
        buildplan -> Jsonb,
    }
}

joinable!(job_envs -> envvars (env_id));
joinable!(job_envs -> jobs (job_id));
joinable!(job_input_artifacts -> artifacts (artifact_id));
joinable!(job_input_artifacts -> jobs (job_id));
joinable!(jobs -> artifacts (artifact_id));
joinable!(jobs -> endpoints (endpoint_id));
joinable!(jobs -> images (image_id));
joinable!(jobs -> packages (package_id));
joinable!(jobs -> submits (submit_id));
joinable!(submit_envs -> envvars (env_id));
joinable!(submit_envs -> submits (submit_id));
joinable!(submits -> githashes (repo_hash_id));
joinable!(submits -> images (requested_image_id));
joinable!(submits -> packages (requested_package_id));

allow_tables_to_appear_in_same_query!(
    artifacts,
    endpoints,
    envvars,
    githashes,
    images,
    job_envs,
    job_input_artifacts,
    jobs,
    packages,
    submit_envs,
    submits,
);
