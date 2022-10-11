use crate::{api::execute_command, Error};
use ocall_commands::{
    Command, CommandResult, GetIASSocketResult, GetQuoteInput, GetQuoteResult,
    GetReportAttestationStatusInput, GetReportAttestationStatusResult, InitQuoteResult,
    RemoteAttestationCommand, RemoteAttestationResult,
};

pub fn init_quote() -> Result<InitQuoteResult, Error> {
    let cmd = Command::RemoteAttestation(RemoteAttestationCommand::InitQuote);
    if let CommandResult::RemoteAttestation(RemoteAttestationResult::InitQuote(res)) =
        execute_command(cmd)?
    {
        Ok(res)
    } else {
        unreachable!()
    }
}

pub fn get_ias_socket() -> Result<GetIASSocketResult, Error> {
    let cmd = Command::RemoteAttestation(RemoteAttestationCommand::GetIASSocket);
    if let CommandResult::RemoteAttestation(RemoteAttestationResult::GetIASSocket(res)) =
        execute_command(cmd)?
    {
        Ok(res)
    } else {
        unreachable!()
    }
}

pub fn get_quote(input: GetQuoteInput) -> Result<GetQuoteResult, Error> {
    let cmd = Command::RemoteAttestation(RemoteAttestationCommand::GetQuote(input));
    if let CommandResult::RemoteAttestation(RemoteAttestationResult::GetQuote(res)) =
        execute_command(cmd)?
    {
        Ok(res)
    } else {
        unreachable!()
    }
}

pub fn get_report_attestation_status(
    input: GetReportAttestationStatusInput,
) -> Result<GetReportAttestationStatusResult, Error> {
    let cmd =
        Command::RemoteAttestation(RemoteAttestationCommand::GetReportAttestationStatus(input));
    if let CommandResult::RemoteAttestation(RemoteAttestationResult::GetReportAttestationStatus(
        res,
    )) = execute_command(cmd)?
    {
        Ok(res)
    } else {
        unreachable!()
    }
}
