param([Parameter(Mandatory)][string]$Executable, [Parameter(Mandatory)][string]$Report, [string]$Models)
$ErrorActionPreference = 'Stop'
# Process creation and Filtering Platform Connection auditing must already be active.
# The workflow enables both policies only in its disposable Windows VM.
$started = Get-Date
$launch = [Diagnostics.ProcessStartInfo]::new($Executable)
$launch.UseShellExecute = $false
$launch.ArgumentList.Add('--check-offline')
$launch.ArgumentList.Add($Report)
if ($Models) { $launch.ArgumentList.Add($Models) }
$process = [Diagnostics.Process]::Start($launch)
if (-not $process.WaitForExit(600000)) {
    $process.Kill($true)
    throw 'Installed check timed out'
}
Start-Sleep -Seconds 2
$events = Get-WinEvent -FilterHashtable @{ LogName='Security'; ID=4688,5156,5157; StartTime=$started } -Oldest
$family = [Collections.Generic.HashSet[long]]::new()
[void]$family.Add([long]$process.Id)
$network = [Collections.Generic.List[object]]::new()
$seenCodec = $false
foreach ($event in $events) {
    $fields = @{}
    foreach ($field in ([xml]$event.ToXml()).Event.EventData.Data) { $fields[$field.Name] = $field.'#text' }
    if ($event.Id -eq 4688) {
        if ($family.Contains([Convert]::ToInt64($fields.ProcessId, 16))) {
            [void]$family.Add([Convert]::ToInt64($fields.NewProcessId, 16))
            if ($fields.NewProcessName -like '*\ffmpeg.exe') { $seenCodec = $true }
        }
    } elseif ($family.Contains([long]$fields.ProcessID)) {
        $network.Add($fields)
    }
}
if (-not $seenCodec) { throw 'Missing audit records for bundled FFmpeg child' }
if (-not ($events | Where-Object { $_.Id -in 5156,5157 } | Select-Object -First 1)) {
    throw 'Windows Filtering Platform provided no control events'
}
foreach ($connection in $network) {
    $address = [Net.IPAddress]::Parse($connection.DestAddress)
    if ($address.IsIPv4MappedToIPv6) { $address = $address.MapToIPv4() }
    if (-not [Net.IPAddress]::IsLoopback($address)) {
        throw "External IP destination observed for $($connection.Application): $address"
    }
}
Write-Output "Windows Filtering Platform checked $($family.Count) processes and $($network.Count) connections."
if ($process.ExitCode -ne 0) { throw "Installed check failed with exit code $($process.ExitCode)" }
