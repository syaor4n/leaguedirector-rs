use serde_json::{json, Value};

pub fn cinematic() -> Value {
    json!({
        "fogOfWar": false,
        "interfaceAll": false,
        "interfaceReplay": false,
        "interfaceMinimap": false,
        "interfaceTimeline": false,
        "interfaceScore": false,
        "interfaceScoreboard": false,
        "interfaceChat": false,
        "interfaceAnnounce": false,
        "interfaceKillCallouts": false,
        "interfaceQuests": false,
        "interfaceFrames": false,
        "interfaceTarget": false,
        "healthBarChampions": false,
        "healthBarStructures": false,
        "healthBarWards": false,
        "healthBarPets": false,
        "healthBarMinions": false,
        "depthOfFieldEnabled": true,
        "floatingText": false,
    })
}

pub fn gameplay() -> Value {
    json!({
        "fogOfWar": true,
        "interfaceAll": true,
        "interfaceReplay": true,
        "interfaceMinimap": true,
        "interfaceTimeline": true,
        "interfaceScore": true,
        "interfaceScoreboard": true,
        "interfaceChat": true,
        "interfaceAnnounce": true,
        "interfaceKillCallouts": true,
        "interfaceQuests": true,
        "healthBarChampions": true,
        "healthBarStructures": true,
        "healthBarWards": true,
        "healthBarPets": true,
        "healthBarMinions": true,
        "depthOfFieldEnabled": false,
        "floatingText": true,
    })
}

pub fn broadcast() -> Value {
    json!({
        "fogOfWar": false,
        "interfaceAll": true,
        "interfaceReplay": false,
        "interfaceMinimap": true,
        "interfaceTimeline": false,
        "interfaceScore": true,
        "interfaceScoreboard": true,
        "interfaceChat": false,
        "interfaceAnnounce": false,
        "interfaceKillCallouts": true,
        "interfaceQuests": false,
        "healthBarChampions": true,
        "healthBarStructures": false,
        "healthBarWards": false,
        "depthOfFieldEnabled": false,
        "floatingText": false,
    })
}
