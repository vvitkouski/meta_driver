$rootCert = New-SelfSignedCertificate -CertStoreLocation Cert:\CurrentUser\My -DnsName "RootCA" -TextExtension @("2.5.29.19={text}CA=true") -KeyUsage CertSign,CrlSign,DigitalSignature
$rootCertPassword = ConvertTo-SecureString -String "password" -Force -AsPlainText
$driverCert = New-SelfSignedCertificate -CertStoreLocation Cert:\LocalMachine\My -DnsName "SignedByRootCA" -KeyExportPolicy Exportable -KeyLength 2048 -KeyUsage DigitalSignature,KeyEncipherment -Signer $rootCert
$driverCertPath = Join-Path -Path 'cert:\LocalMachine\My\' -ChildPath "$($driverCert.Thumbprint)"
Export-PfxCertificate -Cert $driverCertPath -FilePath driver_cert.pfx -Password $rootCertPassword
Export-Certificate -Cert $driverCertPath -FilePath driver_cert.crt