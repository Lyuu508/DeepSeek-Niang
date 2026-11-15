param([ValidateSet('Generate','Check','Sign','Verify')][string]$Action)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw 'Test certificate operations require an ephemeral GitHub-hosted runner' }
$Password = ConvertTo-SecureString $env:TEST_WINDOWS_PASSWORD -AsPlainText -Force
$Certificate = $null
try {
    if ($Action -eq 'Generate') {
        $Certificate = New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=DeepSeek Niang Development Test' -CertStoreLocation 'Cert:\CurrentUser\My' -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 -NotAfter (Get-Date).AddYears(10)
        Export-PfxCertificate -Cert $Certificate -FilePath $env:TEST_WINDOWS_PFX -Password $Password -CryptoAlgorithmOption AES256_SHA256 | Out-Null
    } else {
        $Certificate = Import-PfxCertificate -FilePath $env:TEST_WINDOWS_PFX -Password $Password -CertStoreLocation 'Cert:\CurrentUser\My'
    }
    if (-not $Certificate.HasPrivateKey -or $Certificate.Issuer -ne $Certificate.Subject) { throw 'Expected the persisted self-signed test identity' }
    $Usage = @($Certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' })
    if ($Usage.Count -ne 1 -or '1.3.6.1.5.5.7.3.3' -notin @($Usage[0].EnhancedKeyUsages | ForEach-Object { $_.Value })) { throw 'Code signing EKU missing' }
    Export-Certificate -Cert $Certificate -FilePath $env:TEST_WINDOWS_CER | Out-Null
    $Private = [System.Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($Certificate)
    $Public = [System.Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPublicKey($Certificate)
    try {
        $Probe = [System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32)
        $Proof = $Private.SignData($Probe, [System.Security.Cryptography.HashAlgorithmName]::SHA256, [System.Security.Cryptography.RSASignaturePadding]::Pkcs1)
        if (-not $Public.VerifyData($Probe,$Proof,[System.Security.Cryptography.HashAlgorithmName]::SHA256,[System.Security.Cryptography.RSASignaturePadding]::Pkcs1)) { throw 'Code signing private-key proof failed' }
    } finally { $Private.Dispose(); $Public.Dispose() }
    if ($Action -in @('Sign','Verify')) {
        # Trust only in this disposable runner to check the PE digest and chain.
        # Nothing is installed in the end user's certificate store.
        Import-Certificate -FilePath $env:TEST_WINDOWS_CER -CertStoreLocation 'Cert:\LocalMachine\Root' | Out-Null
        try {
            if ($Action -eq 'Sign') { Set-AuthenticodeSignature -LiteralPath $env:TEST_WINDOWS_INSTALLER -Certificate $Certificate -HashAlgorithm SHA256 | Out-Null }
            $Signed = Get-AuthenticodeSignature -LiteralPath $env:TEST_WINDOWS_INSTALLER
            if ($Signed.Status -ne 'Valid' -or $Signed.SignerCertificate.Thumbprint -ne $Certificate.Thumbprint) { throw 'Authenticode verification failed' }
        } finally { Remove-Item ('Cert:\LocalMachine\Root\' + $Certificate.Thumbprint) -ErrorAction SilentlyContinue }
    }
} catch {
    Write-Output ('CERTIFICATE_STAGE_ERROR:' + $Action + ':' + $_.InvocationInfo.ScriptLineNumber + ':' + $_.Exception.GetType().Name)
    throw
} finally {
    if ($Certificate) { Remove-Item ('Cert:\CurrentUser\My\' + $Certificate.Thumbprint) -ErrorAction SilentlyContinue }
}
