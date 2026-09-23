$ErrorActionPreference = 'Stop'

$packageName = 'linvclipboard'
$url = 'https://github.com/DreamerX00/LinVClipBoard/releases/download/v3.2.0/LinVClipBoard_3.2.0_x64-setup.exe'
$checksum = 'SHA256_HASH'
$checksumType = 'sha256'

$packageArgs = @{
  packageName   = $packageName
  fileType      = 'exe'
  url           = $url
  checksum      = $checksum
  checksumType  = $checksumType
  silentArgs    = '/S'
  validExitCodes= @(0)
}

Install-ChocolateyPackage @packageArgs
