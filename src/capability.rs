// src/capability.rs — 바디 능력(capability) 레이어
//
// 같은 SDK라도 바디마다 노출하는 속성 코드가 다르다 (A7C는 자이로 레벨·Creative Look·
// 벌브 타이머·AF영역 device property 등을 노출하지 않음). 연결 직후 get_all로 실제
// 노출 코드를 수집해 두면, 서버/프론트가 "이 바디가 지원하는가"를 모델 하드코딩 없이
// 질의할 수 있다.

use std::collections::BTreeSet;

use crate::error::SdkResult;
use crate::properties;

/// 연결된 바디가 노출하는 속성 코드 집합 + 모델명.
#[derive(Debug, Clone)]
pub struct Capabilities {
    pub model: String,
    pub supported: BTreeSet<u32>,
}

impl Capabilities {
    /// 연결된 핸들에서 노출 속성 코드를 수집한다.
    /// `model`은 enumerate 단계에서 얻은 실제 모델명(get_all에는 없음).
    pub fn probe(handle: i64, model: String) -> SdkResult<Self> {
        let supported = properties::get_all(handle)?
            .into_iter()
            .map(|p| p.code)
            .collect();
        Ok(Capabilities { model, supported })
    }

    /// 해당 속성 코드를 이 바디가 노출하는가.
    pub fn has(&self, code: u32) -> bool {
        self.supported.contains(&code)
    }
}
