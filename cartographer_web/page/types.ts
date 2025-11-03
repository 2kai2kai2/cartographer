export interface EU4SaveGame {
    all_nations: Map<string, object>;
    player_tags: Map<string, string>;
    provinces: Map<number, string>;
    dlc: string[];
    great_powers: string[];
    date: object;
    multiplayer: boolean;
    age?: string;
    hre?: string;
    china?: string;
    crusade?: string;
    player_wars: object[];
    game_mod: string;
}

export interface StellarisSaveGame {
    all_nations: Map<number, object>;
    player_tags: Map<number, string>;
    galactic_objects: object[];
    dlc: string[];
    date: object;
    multiplayer: boolean;
    galaxy_radius: number;
    game_mod: string;
}
