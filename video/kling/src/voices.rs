use golem_video::exports::golem::video_generation::types::{VoiceInfo, VoiceLanguage};

/// Voice data for Kling lip-sync functionality
/// Data sourced from Kling API documentation
/// https://docs.qingque.cn/s/home/eZQDvafJ4vXQkP8T9ZPvmye8S?identityId=2E3S0NySBQy
/// It isnt possible to do this dynamically, or have preview urls
/// Even though this is from official documentation, some of them are not valid voice-ids
fn get_chinese_voices() -> Vec<VoiceInfo> {
    vec![
        VoiceInfo {
            voice_id: "genshin_vindi2".to_string(),
            name: "阳光少年".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "zhinen_xuesheng".to_string(),
            name: "懂事小弟".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "tiyuxi_xuedi".to_string(),
            name: "运动少年".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_shatang".to_string(),
            name: "青春少女".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "genshin_klee2".to_string(),
            name: "温柔小妹".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "genshin_kirara".to_string(),
            name: "元气少女".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_kaiya".to_string(),
            name: "阳光男生".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "tiexin_nanyou".to_string(),
            name: "幽默小哥".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_chenjiahao_712".to_string(),
            name: "文艺小哥".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "girlfriend_1_speech02".to_string(),
            name: "甜美邻家".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "chat1_female_new-3".to_string(),
            name: "温柔姐姐".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "girlfriend_2_speech02".to_string(),
            name: "职场女青".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "cartoon-boy-07".to_string(),
            name: "活泼男童".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "cartoon-girl-01".to_string(),
            name: "俏皮女童".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_huangyaoshi_712".to_string(),
            name: "稳重老爸".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "you_pingjing".to_string(),
            name: "温柔妈妈".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_laoguowang_712".to_string(),
            name: "严肃上司".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "chengshu_jiejie".to_string(),
            name: "优雅贵妇".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "zhuxi_speech02".to_string(),
            name: "慈祥爷爷".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "uk_oldman3".to_string(),
            name: "唠叨爷爷".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "laopopo_speech02".to_string(),
            name: "唠叨奶奶".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "heainainai_speech02".to_string(),
            name: "和蔼奶奶".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "dongbeilaotie_speech02".to_string(),
            name: "东北老铁".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "chongqingxiaohuo_speech02".to_string(),
            name: "重庆小伙".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "chuanmeizi_speech02".to_string(),
            name: "四川妹子".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "chaoshandashu_speech02".to_string(),
            name: "潮汕大叔".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_taiwan_man2_speech02".to_string(),
            name: "台湾男生".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "xianzhanggui_speech02".to_string(),
            name: "西安掌柜".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "tianjinjiejie_speech02".to_string(),
            name: "天津姐姐".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "diyinnansang_DB_CN_M_04-v2".to_string(),
            name: "新闻播报男".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "yizhipiannan-v1".to_string(),
            name: "译制片男".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "guanxiaofang-v2".to_string(),
            name: "元气少女".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "tianmeixuemei-v1".to_string(),
            name: "撒娇女友".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "daopianyansang-v1".to_string(),
            name: "刀片烟嗓".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "mengwa-v1".to_string(),
            name: "乖巧正太".to_string(),
            language: VoiceLanguage::Zh,
            preview_url: None,
        },
    ]
}

fn get_english_voices() -> Vec<VoiceInfo> {
    vec![
        VoiceInfo {
            voice_id: "genshin_vindi2".to_string(),
            name: "Sunny".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "zhinen_xuesheng".to_string(),
            name: "Sage".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "AOT".to_string(),
            name: "Ace".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_shatang".to_string(),
            name: "Blossom".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "genshin_klee2".to_string(),
            name: "Peppy".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "genshin_kirara".to_string(),
            name: "Dove".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_kaiya".to_string(),
            name: "Shine".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "oversea_male1".to_string(),
            name: "Anchor".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_chenjiahao_712".to_string(),
            name: "Lyric".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "girlfriend_4_speech02".to_string(),
            name: "Melody".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "chat1_female_new-3".to_string(),
            name: "Tender".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "chat_0407_5-1".to_string(),
            name: "Siren".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "cartoon-boy-07".to_string(),
            name: "Zippy".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "uk_boy1".to_string(),
            name: "Bud".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "cartoon-girl-01".to_string(),
            name: "Sprite".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "PeppaPig_platform".to_string(),
            name: "Candy".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_huangzhong_712".to_string(),
            name: "Beacon".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_huangyaoshi_712".to_string(),
            name: "Rock".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "ai_laoguowang_712".to_string(),
            name: "Titan".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "chengshu_jiejie".to_string(),
            name: "Grace".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "you_pingjing".to_string(),
            name: "Helen".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "calm_story1".to_string(),
            name: "Lore".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "uk_man2".to_string(),
            name: "Crag".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "laopopo_speech02".to_string(),
            name: "Prattle".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "heainainai_speech02".to_string(),
            name: "Hearth".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "reader_en_m-v1".to_string(),
            name: "The Reader".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
        VoiceInfo {
            voice_id: "commercial_lady_en_f-v1".to_string(),
            name: "Commercial Lady".to_string(),
            language: VoiceLanguage::En,
            preview_url: None,
        },
    ]
}

/// Get all voices for a specific language, or all voices if language is None
pub fn get_voices(language: Option<String>) -> Vec<VoiceInfo> {
    match language.as_deref() {
        Some("zh") => get_chinese_voices(),
        Some("en") => get_english_voices(),
        Some(_) => Vec::new(), // Unknown language
        None => {
            let mut all_voices = Vec::new();
            all_voices.extend(get_chinese_voices());
            all_voices.extend(get_english_voices());
            all_voices
        }
    }
}
