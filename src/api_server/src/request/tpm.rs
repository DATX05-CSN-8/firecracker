use logger::{METRICS, IncMetric};
use vmm::vmm_config::tpm::TpmDeviceConfig;
use crate::request::Body;
use crate::parsed_request::{Error, ParsedRequest};
use super::super::VmmAction;


pub(crate) fn parse_put_tpm(body: &Body) -> Result<ParsedRequest, Error> {
    METRICS.put_api_requests.tpm_count.inc();
    let tpm_cfg = serde_json::from_slice::<TpmDeviceConfig>(body.raw())
        .map_err(|err| {
            METRICS.put_api_requests.tpm_fails.inc();
            err
        })?;
    let parsed_req = ParsedRequest::new_sync(VmmAction::SetTpmDevice(tpm_cfg));
    Ok(parsed_req)
}