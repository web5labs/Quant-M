use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NumericConfidence {
    Exact,
    WithinTolerance,
    BoundaryAmbiguous,
    Mismatch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdRelation {
    Below,
    Equal,
    Above,
    NearBoundary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThresholdComparison {
    pub value: f64,
    pub threshold: f64,
    pub epsilon: f64,
    pub relation: ThresholdRelation,
    pub confidence: NumericConfidence,
}

pub fn compare_threshold(value: f64, threshold: f64, epsilon: f64) -> ThresholdComparison {
    let distance = (value - threshold).abs();
    let (relation, confidence) = if distance <= epsilon {
        (
            ThresholdRelation::NearBoundary,
            NumericConfidence::BoundaryAmbiguous,
        )
    } else if value < threshold {
        (ThresholdRelation::Below, NumericConfidence::Exact)
    } else if value > threshold {
        (ThresholdRelation::Above, NumericConfidence::Exact)
    } else {
        (ThresholdRelation::Equal, NumericConfidence::Exact)
    };
    ThresholdComparison {
        value,
        threshold,
        epsilon,
        relation,
        confidence,
    }
}
