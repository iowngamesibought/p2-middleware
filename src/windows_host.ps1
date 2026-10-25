$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    $a = $request.args
    $admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    function Checked-Index($index) {
        $n = [uint32]$index
        if ($n -eq 0) { throw 'Invalid interface' }
        return $n
    }
    $result = switch ($request.operation) {
        'probe_network_snapshot' {
            $i=Checked-Index $a.index
            $iface=Get-NetIPInterface -InterfaceIndex $i -AddressFamily IPv4 -PolicyStore ActiveStore
            $ips=@(Get-NetIPAddress -InterfaceIndex $i -AddressFamily IPv4 | Sort-Object IPAddress | Select-Object IPAddress,PrefixLength,PrefixOrigin,SuffixOrigin,SkipAsSource)
            $routes=@(Get-NetRoute -InterfaceIndex $i -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' -PolicyStore ActiveStore | Sort-Object NextHop,RouteMetric | Select-Object NextHop,RouteMetric)
            $dns=@((Get-DnsClientServerAddress -InterfaceIndex $i -AddressFamily IPv4).ServerAddresses)
            @{dhcp=[string]$iface.Dhcp;forwarding=[string]$iface.Forwarding;addresses=$ips;defaults=$routes;dns=$dns};break
        }
        'interface' {
            $i=Checked-Index $a.index
            $ips=@(Get-NetIPAddress -InterfaceIndex $i -AddressFamily IPv4 | Where-Object IPAddress -eq ([string]$a.local))
            if($ips.Count -ne 1){throw 'Interface address changed'}
            $routes=@(Get-NetRoute -InterfaceIndex $i -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' -PolicyStore ActiveStore | Sort-Object RouteMetric)
            if($routes.Count -eq 0){throw 'Missing default route'}
            $iface=Get-NetIPInterface -InterfaceIndex $i -AddressFamily IPv4 -PolicyStore ActiveStore
            @{address=$ips[0].IPAddress;prefix=$ips[0].PrefixLength;gateway=$routes[0].NextHop;forwarding=($iface.Forwarding -eq 'Enabled')}; break
        }
        'secure_store' {
            if(-not $admin){throw 'Administrator required'}
            $folder=Join-Path ([Environment]::GetFolderPath('CommonApplicationData')) 'P2Middleware'
            if(-not [IO.Directory]::Exists($folder)){
                $security=[Security.AccessControl.DirectorySecurity]::new()
                $security.SetSecurityDescriptorSddlForm('O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)')
                [IO.Directory]::CreateDirectory($folder,$security) | Out-Null
            }
            if(([IO.File]::GetAttributes($folder) -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'Reparse directory refused'}
            $acl=Get-Acl -LiteralPath $folder
            if($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne 'S-1-5-32-544' -or -not $acl.AreAccessRulesProtected){throw 'Untrusted store owner or inheritance'}
            $rules=@($acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier]))
            if($rules.Count -ne 2){throw 'Unexpected store permissions'}
            foreach($rule in $rules){
                if($rule.IdentityReference.Value -notin @('S-1-5-18','S-1-5-32-544') -or $rule.AccessControlType -ne 'Allow' -or
                    $rule.FileSystemRights -ne 'FullControl' -or $rule.IsInherited -or $rule.InheritanceFlags -ne 'ContainerInherit, ObjectInherit') {throw 'Unsafe store permissions'}
            }
            $folder;break
        }
        'forwarding' {
            if(-not $admin){throw 'Administrator required'}
            $i=Checked-Index $a.index
            if(@(Get-NetIPAddress -InterfaceIndex $i -AddressFamily IPv4 | Where-Object IPAddress -eq ([string]$a.local)).Count -ne 1){throw 'Interface changed'}
            $value=if([bool]$a.enabled){'Enabled'}else{'Disabled'}
            Set-NetIPInterface -InterfaceIndex $i -AddressFamily IPv4 -PolicyStore ActiveStore -Forwarding $value
            if((Get-NetIPInterface -InterfaceIndex $i -AddressFamily IPv4 -PolicyStore ActiveStore).Forwarding -ne $value){throw 'Forwarding verification failed'}
            $true;break
        }
        default { throw 'Unknown host operation' }
    }
    ConvertTo-Json -InputObject $result -Compress -Depth 8
} catch {
    # Do not echo arbitrary Windows output, paths or exception contents to the desktop.
    exit 1
}
