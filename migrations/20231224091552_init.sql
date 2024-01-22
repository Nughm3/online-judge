CREATE TABLE IF NOT EXISTS users (
    id        INTEGER PRIMARY KEY NOT NULL,
    username  TEXT NOT NULL UNIQUE,
    password  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS admins (
    id       INTEGER PRIMARY KEY NOT NULL,
    user_id  INTEGER NOT NULL UNIQUE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS sessions (
    id            INTEGER PRIMARY KEY NOT NULL,
    contest_name  TEXT NOT NULL,
    contest_path  TEXT NOT NULL,
    start         DATETIME,
    end           DATETIME
);

CREATE TABLE IF NOT EXISTS submissions (
    id             INTEGER PRIMARY KEY NOT NULL,
    user_id        INTEGER NOT NULL,
    session_id     INTEGER NOT NULL,
    task           INTEGER NOT NULL,
    datetime       DATETIME NOT NULL,
    code           TEXT NOT NULL,
    language       TEXT NOT NULL,
    verdict        TEXT NOT NULL,
    score          INTEGER NOT NULL,
    compile_error  TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS subtasks (
    id             INTEGER PRIMARY KEY NOT NULL,
    submission_id  INTEGER NOT NULL,
    subtask        INTEGER NOT NULL,
    verdict        TEXT NOT NULL,
    score          INTEGER NOT NULL,
    FOREIGN KEY (submission_id) REFERENCES submissions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tests (
    id          INTEGER PRIMARY KEY NOT NULL,
    subtask_id  INTEGER NOT NULL,
    test        INTEGER NOT NULL,
    memory      INTEGER,
    time        INTEGER,
    verdict     TEXT NOT NULL,
    score       INTEGER NOT NULL,
    FOREIGN KEY (subtask_id) REFERENCES subtasks(id) ON DELETE CASCADE
);
