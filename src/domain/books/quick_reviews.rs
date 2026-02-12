use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuickReview {
    LovedIt,
    PageTurner,
    ThoughtProvoking,
    CouldntPutDown,
    GreatCharacters,
    Funny,
    Moving,
    LaughedOutLoud,
    Relatable,
    QuickRead,
    SlowBurn,
    Dense,
    PredictablePlot,
    DisappointingEnding,
    UnrelatableCharacters,
    Forgettable,
    TooLong,
    OddPov,
    Overrated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

impl QuickReview {
    pub fn label(self) -> &'static str {
        match self {
            Self::LovedIt => "Loved it",
            Self::PageTurner => "Page-turner",
            Self::ThoughtProvoking => "Thought-provoking",
            Self::CouldntPutDown => "Couldn't put down",
            Self::GreatCharacters => "Great characters",
            Self::Funny => "Funny",
            Self::Moving => "Moving",
            Self::LaughedOutLoud => "Laughed out loud",
            Self::Relatable => "Relatable",
            Self::QuickRead => "Quick read",
            Self::SlowBurn => "Slow burn",
            Self::Dense => "Dense",
            Self::PredictablePlot => "Predictable plot",
            Self::DisappointingEnding => "Disappointing ending",
            Self::UnrelatableCharacters => "Unrelatable characters",
            Self::Forgettable => "Forgettable",
            Self::TooLong => "Too long",
            Self::OddPov => "Odd POV",
            Self::Overrated => "Overrated",
        }
    }

    pub fn form_value(self) -> &'static str {
        match self {
            Self::LovedIt => "loved-it",
            Self::PageTurner => "page-turner",
            Self::ThoughtProvoking => "thought-provoking",
            Self::CouldntPutDown => "couldnt-put-down",
            Self::GreatCharacters => "great-characters",
            Self::Funny => "funny",
            Self::Moving => "moving",
            Self::LaughedOutLoud => "laughed-out-loud",
            Self::Relatable => "relatable",
            Self::QuickRead => "quick-read",
            Self::SlowBurn => "slow-burn",
            Self::Dense => "dense",
            Self::PredictablePlot => "predictable-plot",
            Self::DisappointingEnding => "disappointing-ending",
            Self::UnrelatableCharacters => "unrelatable-characters",
            Self::Forgettable => "forgettable",
            Self::TooLong => "too-long",
            Self::OddPov => "odd-pov",
            Self::Overrated => "overrated",
        }
    }

    pub fn from_str_value(s: &str) -> Option<Self> {
        match s {
            "loved-it" | "Loved it" => Some(Self::LovedIt),
            "page-turner" | "Page-turner" => Some(Self::PageTurner),
            "thought-provoking" | "Thought-provoking" => Some(Self::ThoughtProvoking),
            "couldnt-put-down" | "Couldn't put down" => Some(Self::CouldntPutDown),
            "great-characters" | "Great characters" => Some(Self::GreatCharacters),
            "funny" | "Funny" => Some(Self::Funny),
            "moving" | "Moving" => Some(Self::Moving),
            "laughed-out-loud" | "Laughed out loud" => Some(Self::LaughedOutLoud),
            "relatable" | "Relatable" => Some(Self::Relatable),
            "quick-read" | "Quick read" => Some(Self::QuickRead),
            "slow-burn" | "Slow burn" => Some(Self::SlowBurn),
            "dense" | "Dense" => Some(Self::Dense),
            "predictable-plot" | "Predictable plot" => Some(Self::PredictablePlot),
            "disappointing-ending" | "Disappointing ending" => Some(Self::DisappointingEnding),
            "unrelatable-characters" | "Unrelatable characters" => {
                Some(Self::UnrelatableCharacters)
            }
            "forgettable" | "Forgettable" => Some(Self::Forgettable),
            "too-long" | "Too long" => Some(Self::TooLong),
            "odd-pov" | "Odd POV" => Some(Self::OddPov),
            "overrated" | "Overrated" => Some(Self::Overrated),
            _ => None,
        }
    }

    pub fn sentiment(self) -> Sentiment {
        match self {
            Self::LovedIt
            | Self::PageTurner
            | Self::ThoughtProvoking
            | Self::CouldntPutDown
            | Self::GreatCharacters
            | Self::Funny
            | Self::Moving
            | Self::LaughedOutLoud
            | Self::Relatable => Sentiment::Positive,
            Self::QuickRead | Self::SlowBurn | Self::Dense => Sentiment::Neutral,
            Self::PredictablePlot
            | Self::DisappointingEnding
            | Self::UnrelatableCharacters
            | Self::Forgettable
            | Self::TooLong
            | Self::OddPov
            | Self::Overrated => Sentiment::Negative,
        }
    }

    pub fn is_positive(self) -> bool {
        self.sentiment() == Sentiment::Positive
    }

    pub fn is_neutral(self) -> bool {
        self.sentiment() == Sentiment::Neutral
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::LovedIt,
            Self::PageTurner,
            Self::ThoughtProvoking,
            Self::CouldntPutDown,
            Self::GreatCharacters,
            Self::Funny,
            Self::Moving,
            Self::LaughedOutLoud,
            Self::Relatable,
            Self::QuickRead,
            Self::SlowBurn,
            Self::Dense,
            Self::PredictablePlot,
            Self::DisappointingEnding,
            Self::UnrelatableCharacters,
            Self::Forgettable,
            Self::TooLong,
            Self::OddPov,
            Self::Overrated,
        ]
    }
}
