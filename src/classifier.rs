/// Domain and network traffic classifier.
/// Maps domain names to known services (YouTube, WhatsApp, Netflix, Steam, etc.)
/// and service categories (Streaming, Social, Gaming, Work).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceCategory {
    VideoStreaming,
    SocialChat,
    MusicStreaming,
    Gaming,
    WorkCloud,
    WebBrowsing,
    SystemOther,
}

impl ServiceCategory {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::VideoStreaming => "🎬",
            Self::SocialChat => "💬",
            Self::MusicStreaming => "🎵",
            Self::Gaming => "🎮",
            Self::WorkCloud => "💼",
            Self::WebBrowsing => "🌐",
            Self::SystemOther => "⚙️",
        }
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::VideoStreaming => "Video",
            Self::SocialChat => "Social/Chat",
            Self::MusicStreaming => "Music",
            Self::Gaming => "Gaming",
            Self::WorkCloud => "Work/Cloud",
            Self::WebBrowsing => "Browsing",
            Self::SystemOther => "Network",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceMatch {
    pub service_name: String,
    pub category: ServiceCategory,
}

/// Classify a domain name into a human-readable service and category.
pub fn classify_domain(domain: &str) -> ServiceMatch {
    let d = domain.to_lowercase();

    // ── Video Streaming ──
    if d.contains("youtube.com") || d.contains("googlevideo.com") || d.contains("ytimg.com") || d.contains("youtu.be") {
        return ServiceMatch { service_name: "YouTube".into(), category: ServiceCategory::VideoStreaming };
    }
    if d.contains("netflix.com") || d.contains("nflxvideo.net") || d.contains("nflxext.com") {
        return ServiceMatch { service_name: "Netflix".into(), category: ServiceCategory::VideoStreaming };
    }
    if d.contains("primevideo.com") || d.contains("aiv-cdn.net") {
        return ServiceMatch { service_name: "Prime Video".into(), category: ServiceCategory::VideoStreaming };
    }
    if d.contains("hotstar.com") || d.contains("disneyplus.com") || d.contains("starott.com") {
        return ServiceMatch { service_name: "Disney+/Hotstar".into(), category: ServiceCategory::VideoStreaming };
    }
    if d.contains("twitch.tv") || d.contains("ttvnw.net") {
        return ServiceMatch { service_name: "Twitch".into(), category: ServiceCategory::VideoStreaming };
    }
    if d.contains("vimeo.com") {
        return ServiceMatch { service_name: "Vimeo".into(), category: ServiceCategory::VideoStreaming };
    }

    // ── Social & Messaging ──
    if d.contains("whatsapp.com") || d.contains("whatsapp.net") || d.contains("wa.me") {
        return ServiceMatch { service_name: "WhatsApp".into(), category: ServiceCategory::SocialChat };
    }
    if d.contains("instagram.com") || d.contains("cdninstagram.com") {
        return ServiceMatch { service_name: "Instagram".into(), category: ServiceCategory::SocialChat };
    }
    if d.contains("tiktok.com") || d.contains("byteoversea.com") || d.contains("ibytedtos.com") {
        return ServiceMatch { service_name: "TikTok".into(), category: ServiceCategory::SocialChat };
    }
    if d.contains("telegram.org") || d.contains("t.me") || d.contains("telesco.pe") {
        return ServiceMatch { service_name: "Telegram".into(), category: ServiceCategory::SocialChat };
    }
    if d.contains("facebook.com") || d.contains("fbcdn.net") || d.contains("messenger.com") {
        return ServiceMatch { service_name: "Facebook".into(), category: ServiceCategory::SocialChat };
    }
    if d.contains("twitter.com") || d.contains("x.com") || d.contains("twimg.com") {
        return ServiceMatch { service_name: "X (Twitter)".into(), category: ServiceCategory::SocialChat };
    }
    if d.contains("reddit.com") || d.contains("redd.it") || d.contains("redditmedia.com") {
        return ServiceMatch { service_name: "Reddit".into(), category: ServiceCategory::SocialChat };
    }
    if d.contains("discord.com") || d.contains("discord.gg") || d.contains("discordapp.com") {
        return ServiceMatch { service_name: "Discord".into(), category: ServiceCategory::SocialChat };
    }
    if d.contains("snapchat.com") {
        return ServiceMatch { service_name: "Snapchat".into(), category: ServiceCategory::SocialChat };
    }

    // ── Music ──
    if d.contains("spotify.com") || d.contains("scdn.co") {
        return ServiceMatch { service_name: "Spotify".into(), category: ServiceCategory::MusicStreaming };
    }
    if d.contains("music.apple.com") {
        return ServiceMatch { service_name: "Apple Music".into(), category: ServiceCategory::MusicStreaming };
    }
    if d.contains("soundcloud.com") {
        return ServiceMatch { service_name: "SoundCloud".into(), category: ServiceCategory::MusicStreaming };
    }

    // ── Gaming ──
    if d.contains("steampowered.com") || d.contains("steamcommunity.com") || d.contains("steamcontent.com") {
        return ServiceMatch { service_name: "Steam".into(), category: ServiceCategory::Gaming };
    }
    if d.contains("epicgames.com") || d.contains("unrealengine.com") {
        return ServiceMatch { service_name: "Epic Games".into(), category: ServiceCategory::Gaming };
    }
    if d.contains("playstation.net") || d.contains("playstation.com") || d.contains("sonyentertainmentnetwork.com") {
        return ServiceMatch { service_name: "PlayStation".into(), category: ServiceCategory::Gaming };
    }
    if d.contains("xboxlive.com") || d.contains("xbox.com") {
        return ServiceMatch { service_name: "Xbox Live".into(), category: ServiceCategory::Gaming };
    }
    if d.contains("roblox.com") || d.contains("rbxcdn.com") {
        return ServiceMatch { service_name: "Roblox".into(), category: ServiceCategory::Gaming };
    }
    if d.contains("riotgames.com") || d.contains("leagueoflegends.com") || d.contains("val.pvp.net") {
        return ServiceMatch { service_name: "Riot Games".into(), category: ServiceCategory::Gaming };
    }
    if d.contains("battle.net") || d.contains("blizzard.com") {
        return ServiceMatch { service_name: "Battle.net".into(), category: ServiceCategory::Gaming };
    }

    // ── Work & Cloud ──
    if d.contains("zoom.us") {
        return ServiceMatch { service_name: "Zoom".into(), category: ServiceCategory::WorkCloud };
    }
    if d.contains("teams.microsoft.com") || d.contains("office.com") || d.contains("sharepoint.com") {
        return ServiceMatch { service_name: "MS Teams/Office".into(), category: ServiceCategory::WorkCloud };
    }
    if d.contains("meet.google.com") || d.contains("docs.google.com") || d.contains("drive.google.com") {
        return ServiceMatch { service_name: "Google Workspace".into(), category: ServiceCategory::WorkCloud };
    }
    if d.contains("slack.com") {
        return ServiceMatch { service_name: "Slack".into(), category: ServiceCategory::WorkCloud };
    }
    if d.contains("github.com") || d.contains("githubusercontent.com") {
        return ServiceMatch { service_name: "GitHub".into(), category: ServiceCategory::WorkCloud };
    }
    if d.contains("linkedin.com") {
        return ServiceMatch { service_name: "LinkedIn".into(), category: ServiceCategory::WorkCloud };
    }

    // ── Search & Browsing ──
    if d.contains("google.com") || d.contains("gstatic.com") {
        return ServiceMatch { service_name: "Google".into(), category: ServiceCategory::WebBrowsing };
    }
    if d.contains("wikipedia.org") {
        return ServiceMatch { service_name: "Wikipedia".into(), category: ServiceCategory::WebBrowsing };
    }
    if d.contains("bing.com") {
        return ServiceMatch { service_name: "Bing".into(), category: ServiceCategory::WebBrowsing };
    }
    if d.contains("amazon.com") || d.contains("amazon.in") {
        return ServiceMatch { service_name: "Amazon".into(), category: ServiceCategory::WebBrowsing };
    }

    // ── System & Background ──
    if d.contains("apple.com") || d.contains("icloud.com") {
        return ServiceMatch { service_name: "Apple Cloud".into(), category: ServiceCategory::SystemOther };
    }
    if d.contains("microsoft.com") || d.contains("windowsupdate.com") {
        return ServiceMatch { service_name: "Windows Services".into(), category: ServiceCategory::SystemOther };
    }
    if d.contains("android.com") || d.contains("play.google.com") {
        return ServiceMatch { service_name: "Google Play".into(), category: ServiceCategory::SystemOther };
    }

    // Fallback: extract base domain (e.g., "example.com" from "sub.example.com")
    let parts: Vec<&str> = d.split('.').collect();
    let base_name = if parts.len() >= 2 {
        format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        d.clone()
    };

    ServiceMatch {
        service_name: base_name,
        category: ServiceCategory::WebBrowsing,
    }
}
