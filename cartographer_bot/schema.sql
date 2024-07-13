CREATE TABLE IF NOT EXISTS Games (game_id BIGSERIAL PRIMARY KEY);

CREATE TABLE IF NOT EXISTS Reservations (
    game_id BIGINT REFERENCES Games (game_id) ON DELETE CASCADE,
    user_id BIGINT,
    timestamp TIMESTAMPTZ,
    tag VARCHAR(3),
    PRIMARY KEY (game_id, user_id)
);