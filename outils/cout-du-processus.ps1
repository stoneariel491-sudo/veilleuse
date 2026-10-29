<#
Le cout du verre pour le processus entier, lu par les compteurs de Windows.

    powershell -NoProfile -ExecutionPolicy Bypass -File outils/cout-du-processus.ps1 -Verre <exe> -Avant <exe> -Sortie <cout.txt> [-Series 3]
    powershell -NoProfile -ExecutionPolicy Bypass -File outils/cout-du-processus.ps1 -Verre <integre.exe> -Avant <avant.exe> -Sortie <cout.txt> -Deploye -Page outils/page-calme.py [-Series 3]

SANS -Deploye (l'epreuve, EPREUVE.md §6) : deux binaires, le verre et celui
d'avant, lances sans option, ilot replie, a deux places : la place par defaut
(760, 30), sur le noir de l'horloge, et sur les secondes (1349, 447).

AVEC -Deploye (l'integration, DECISION-APRES-EPREUVE.md A3, pas 8) : les trois
situations de l'arbitre, chacune en series ABBA :

1. l'horloge devant, ilot retire, galet opaque (le lancement) : le binaire
   integre contre celui d'avant, matiere.txt a « opaque », rien devant l'horloge.
   Attendu : ecart sous le seuil aux deux grandeurs, la nuit ne coute rien de plus ;
2. verre choisi, ilot DEPLOYE sur la page calme claire (outils/page-calme.py,
   une fenetre blanche plein ecran, sans rien qui bouge), fenetre calme : le
   binaire integre avec matiere.txt « verre » contre le meme avec « opaque »,
   tous deux `--ilot-deploye`. Attendu : l'ecart GPU tient sous +3,54 points, la
   mediane mesuree du verre sur les secondes (EPREUVE.md §6) ;
3. verre choisi, ilot replie sur la page calme : idem, sans le drapeau.

Chaque lancement : 5 s de mise en route, puis 15 echantillons d'une seconde de

- l'utilisation GPU : la somme des compteurs
  \GPU Engine(*)\Utilization Percentage dont l'instance est pid_<PID>_*engtype_3d ;
- le processeur : le temps consomme par le processus sur la meme fenetre, en
  pourcentage d'un coeur.

Sa sortie standard va dans un journal a cote de la sortie (journaux/<n>.txt) :
c'est la que se lit « retire : l'horloge est devant » ou « parait ».

POURQUOI DES SERIES ABBA. Deux passes successives dans l'ordre verre, verre,
avant, avant ont lu, pour le binaire d'avant a la place par defaut, 3,65 puis
7,71 % d'un coeur (captures/epreuve/cout/cout.txt, cout-2.txt). Le chiffre
absolu d'un lancement ne se reproduit donc pas. Chaque serie mesure une
situation dans l'ordre verre-avant-avant-verre, ou avant-verre-verre-avant en
alternance : la difference des moyennes retranche une derive lineaire au cours
de la serie (RECETTES.md, ABBA).

LES DEUX SEUILS, derives de la mesure elle-meme : dans une serie, les deux
lancements d'un meme binaire donnent l'ecart de ce binaire a lui-meme. Le seuil
de l'epreuve est le plus grand de ces ecarts a cette place, et l'ecart entre
les binaires n'est lu comme reel que si, dans chaque serie, il le depasse. Le
seuil de l'arbitre (A3, JUGE-TECHNICIEN.md M5) est le meme, divise par racine de
deux, parce que la grandeur jugee est une difference de deux moyennes de deux
lancements ; il se compare a la mediane des series.

LES TEMOINS, a reponse connue d'avance :

- la mediane de 1 a 15 melanges vaut 8, celle de 4 1 3 2 vaut 2,5. La premiere
  version prenait l'indice [int](n / 2), que PowerShell arrondit au pair, et ce
  temoin tombait a 9 ;
- le processus qui echantillonne lit son propre GPU, qui reste a zero ;
- le plus petit et le plus grand nombre d'instances 3D trouvees pour le PID
  lance, par echantillon, sont imprimes : un echantillon ou le filtre ne voit
  pas le processus se lirait a zero au plus bas. La premiere version n'imprimait
  que le maximum, aveugle a un echantillon vide.

L'AUTEUR D'ABORD. Un lancement ne part que si aucune entree (clavier, souris)
n'a ete vue depuis 60 s (GetLastInputInfo) : la machine est la sienne, et la
page calme lui prendrait l'ecran. Un lancement pendant lequel une entree
arrive, ou pendant lequel une fenetre autre que celle attendue passe devant
(le journal de l'application le dit : « parait » la ou l'horloge devait etre
devant, « retire » la ou la page calme devait l'etre), est CONTAMINE : il est
compte, dit, et refait, trois fois au plus. La fenetre de cette session
(« Claude ») passe devant a chaque appel d'outil : c'est la contamination la
plus frequente (pas8/essai-1).

Les binaires doivent etre des COPIES hors du depot : le chemin target\release
porte une couche de compatibilite qui y fait refuser la declaration du DPI. Le
dossier de chaque copie sert de dossier de travail. La position de l'ilot est
posee dans %LOCALAPPDATA%\Veilleuse\ilot.txt, et la matiere dans matiere.txt,
puis restaurees ; l'identite avec la sauvegarde est imprimee.
#>
param(
  [Parameter(Mandatory = $true)] [string] $Verre,
  [Parameter(Mandatory = $true)] [string] $Avant,
  [Parameter(Mandatory = $true)] [string] $Sortie,
  [int] $Series = 3,
  [switch] $Deploye,
  [string] $Page = '',
  [string] $Situations = '1,2,3',
  [switch] $Secteur,
  [switch] $Epreuve
)

$Sortie = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Sortie)
New-Item -ItemType Directory -Force (Split-Path $Sortie) | Out-Null
$journaux = Join-Path (Split-Path $Sortie) 'journaux'
New-Item -ItemType Directory -Force $journaux | Out-Null
$pos = Join-Path $env:LOCALAPPDATA 'Veilleuse\ilot.txt'
$mat = Join-Path $env:LOCALAPPDATA 'Veilleuse\matiere.txt'
$sauvegarde = [IO.File]::ReadAllText($pos)
$matiereAvant = if (Test-Path $mat) { [IO.File]::ReadAllText($mat) } else { $null }
$coeurs = [Environment]::ProcessorCount
Add-Type -Namespace Veilleuse -Name Entree -MemberDefinition @'
[System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Sequential)]
public struct LASTINPUTINFO { public uint cbSize; public uint dwTime; }
[System.Runtime.InteropServices.DllImport("user32.dll")] public static extern bool GetLastInputInfo(ref LASTINPUTINFO plii);
[System.Runtime.InteropServices.DllImport("kernel32.dll")] public static extern uint GetTickCount();
'@
Add-Type -Namespace Veilleuse -Name Alimentation -MemberDefinition @'
[System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Sequential)]
public struct SYSTEM_POWER_STATUS { public byte ACLineStatus; public byte BatteryFlag; public byte BatteryLifePercent; public byte SystemStatusFlag; public uint BatteryLifeTime; public uint BatteryFullLifeTime; }
[System.Runtime.InteropServices.DllImport("kernel32.dll")] public static extern bool GetSystemPowerStatus(out SYSTEM_POWER_STATUS s);
'@
# L'alimentation, lue comme l'application la lit (SYSTEM_POWER_STATUS) : le
# troisieme essai a mesure opaque contre opaque parce que l'economiseur etait
# en marche et que rien ne le disait a cote des chiffres.
function Alimentation() {
  $e = New-Object Veilleuse.Alimentation+SYSTEM_POWER_STATUS
  [void][Veilleuse.Alimentation]::GetSystemPowerStatus([ref]$e)
  return ("{0}{1}, economiseur du systeme : {2}" -f $(if ($e.ACLineStatus -eq 1) { 'secteur' } else { 'batterie' }), $(if ($e.BatteryLifePercent -le 100) { ' ' + $e.BatteryLifePercent + ' %' } else { '' }), $(if ($e.SystemStatusFlag -eq 1) { 'EN MARCHE' } else { 'non' }))
}
# Un seul script a la fois : le troisieme essai en a eu trois en meme temps,
# qui s'ecrivaient matiere.txt et se tuaient les processus.
$autres = @(Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe'" | Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -like '*cout-du-processus*' })
if ($autres.Count -gt 0) { throw ("un autre cout-du-processus.ps1 tourne deja (PID " + (($autres | ForEach-Object { $_.ProcessId }) -join ', ') + ") ; un seul a la fois") }
$deja = @(Get-Process | Where-Object { $_.Path -eq $Verre -or $_.Path -eq $Avant -or $_.MainWindowTitle -eq 'Page calme' })
if ($deja.Count -gt 0) { throw ("des processus de la mesure tournent deja (" + (($deja | ForEach-Object { $_.ProcessName + ' ' + $_.Id }) -join ', ') + ") ; les arreter d'abord") }
function DerniereEntree() {
  $lii = New-Object Veilleuse.Entree+LASTINPUTINFO
  $lii.cbSize = [System.Runtime.InteropServices.Marshal]::SizeOf($lii)
  [void][Veilleuse.Entree]::GetLastInputInfo([ref]$lii)
  return [uint32]$lii.dwTime
}
function SecondesSansEntree() { return ([Veilleuse.Entree]::GetTickCount() - (DerniereEntree)) / 1000.0 }
$script:contamines = 0
$lignes = New-Object System.Collections.Generic.List[string]
$script:lancements = 0
$drapeau = Join-Path (Split-Path $Sortie) 'page-devant.drapeau'
function Dire($t) { $lignes.Add($t); Write-Host $t }
function Mediane($valeurs) {
  $tri = @($valeurs | Sort-Object)
  $n = $tri.Count
  if ($n % 2) { return $tri[($n - 1) / 2] }
  return ($tri[$n / 2 - 1] + $tri[$n / 2]) / 2
}
Dire "=== cout du verre, utilisation GPU par processus ($(Get-Date -Format HH:mm:ss)) ; $Series series ABBA par situation ; position sauvegardee : [$($sauvegarde.Trim())] ; matiere sauvegardee : [$(if ($null -ne $matiereAvant) { $matiereAvant.Trim() } else { 'absente' })] ; mode : $(if ($Epreuve) { 'EPREUVE des temoins de la matiere' } elseif ($Deploye) { 'les trois situations de A3' } else { 'les deux places de l''epreuve' }) ; alimentation : $(Alimentation) ; forcage --batterie=secteur sur le binaire integre : $(if ($Secteur) { 'oui (situations 2 et 3)' } else { 'non' })"
$m15 = Mediane (1..15 | Sort-Object { Get-Random })
$m4 = Mediane @(4, 1, 3, 2)
Dire ("  VERDICT temoin de la mediane : {0}  (1 a 15 melanges -> {1}, attendu 8 ; 4 1 3 2 -> {2}, attendu 2,5)" -f $(if ($m15 -eq 8 -and $m4 -eq 2.5) { 'VRAI' } else { 'FAUX' }), $m15, $m4)

# Le parametre des arguments s'appelle $arguments et jamais $args : $args est
# la variable automatique de PowerShell et prime sur un parametre du meme nom,
# qui reste vide (mesure : function f($a, $args) { $args.Count } -> 0). Les
# trois premiers essais ont lance « --ilot-deploye » dans le vide : leurs
# situations « deploye » etaient repliees (aucun journal n'y dit « ne deploye »).
function Mesurer($nom, $exe, $position, $matiere, $arguments, $pageDevant, $matiereEcrite = $null) {
  for ($essai = 1; $essai -le 3; $essai++) {
    # L'auteur d'abord : rien ne part tant qu'il se sert de la machine.
    $attente = 0
    while ((SecondesSansEntree) -lt 60) {
      if ($attente -eq 0) { Dire ("    (l'auteur se sert de la machine : attente qu'aucune entree n'arrive pendant 60 s, {0})" -f (Get-Date -Format HH:mm:ss)) }
      Start-Sleep -Seconds 5
      $attente += 5
    }
    $r = MesurerUneFois $nom $exe $position $matiere $arguments $pageDevant $matiereEcrite
    $entree = $r.EntreePendant
    $contamine = $entree -or $r.Contamine
    if (-not $contamine) { return $r }
    $script:contamines++
    Dire ("    CONTAMINE (essai {0}) : {1}{2} ; refait" -f $essai, $(if ($entree) { 'une entree de l''auteur pendant le lancement' } else { '' }), $(if ($r.Contamine) { ' ' + $r.Pourquoi } else { '' }))
  }
  Dire "    trois essais contamines : le dernier est garde, et dit"
  return $r
}

function MesurerUneFois($nom, $exe, $position, $matiere, $arguments, $pageDevant, $matiereEcrite = $null) {
  [IO.File]::WriteAllText($pos, "$position`n")
  $ecrite = if ($null -ne $matiereEcrite) { $matiereEcrite } else { $matiere }
  if ($null -ne $ecrite) { [IO.File]::WriteAllText($mat, "$ecrite`n") } elseif (Test-Path $mat) { Remove-Item $mat }
  $script:lancements++
  $journal = Join-Path $journaux ("{0:D2}-{1}.txt" -f $script:lancements, ($nom -replace '[^a-z0-9]+', '-'))
  if ($arguments.Count -gt 0) {
    $p = Start-Process -FilePath $exe -ArgumentList $arguments -WorkingDirectory (Split-Path $exe) -PassThru -RedirectStandardOutput $journal
  } else {
    $p = Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe) -PassThru -RedirectStandardOutput $journal
  }
  try {
    if ($pageDevant) {
      # L'horloge vient de prendre le premier plan ; la page calme le reprend
      # (elle envoie une entree nulle, puis SetForegroundWindow) et l'ilot parait.
      Start-Sleep -Milliseconds 1500
      [IO.File]::WriteAllText($drapeau, "devant`n")
      Start-Sleep -Seconds 4
    } else {
      Start-Sleep -Seconds 5
    }
    $entree0 = DerniereEntree
    $cpu0 = (Get-Process -Id $p.Id).TotalProcessorTime.TotalMilliseconds; $t0 = Get-Date
    $ech = Get-Counter -Counter '\GPU Engine(*)\Utilization Percentage' -SampleInterval 1 -MaxSamples 15
    $cpu1 = (Get-Process -Id $p.Id).TotalProcessorTime.TotalMilliseconds; $t1 = Get-Date
    $app = @(); $temoin = @(); $instances = 0; $instancesMin = [int]::MaxValue
    foreach ($s in $ech) {
      $a = @($s.CounterSamples | Where-Object { $_.InstanceName -like "pid_$($p.Id)_*engtype_3d" })
      $instances = [Math]::Max($instances, $a.Count)
      $instancesMin = [Math]::Min($instancesMin, $a.Count)
      $app += [double](($a | Measure-Object CookedValue -Sum).Sum)
      $temoin += [double]((@($s.CounterSamples | Where-Object { $_.InstanceName -like "pid_$($PID)_*" }) | Measure-Object CookedValue -Sum).Sum)
    }
    $cpu = 100.0 * ($cpu1 - $cpu0) / ($t1 - $t0).TotalMilliseconds
    $gpu = Mediane $app
    # Une entree de l'auteur pendant la fenetre d'echantillonnage seulement :
    # avant elle, la page calme a pu passer devant, et ce n'est pas lui.
    $entreePendant = (DerniereEntree) -ne $entree0
    $contamine = $false; $pourquoi = ''
    Dire ("{0,-34} GPU 3D : moyenne {1,6:N2} %  mediane {2,6:N2} %  max {3,6:N2} %  | CPU {4,5:N2} % d'un coeur x {5} | instances 3D du processus {6} (au plus bas {8}) | temoin, GPU de l'echantillonneur : max {7:N2} %" -f $nom, ($app | Measure-Object -Average).Average, $gpu, ($app | Measure-Object -Maximum).Maximum, $cpu, $coeurs, $instances, ($temoin | Measure-Object -Maximum).Maximum, $instancesMin)
    Dire ("    par seconde : " + (($app | ForEach-Object { '{0:N2}' -f $_ }) -join ' ') + " | alimentation : " + (Alimentation))
  } finally {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    if (Test-Path $journal) {
      # Les titres des fenetres de l'auteur ne sont pas a nous : le journal
      # garde ceux de la mesure, et remplace les autres.
      $j = Get-Content $journal -Encoding UTF8
      # Le motif par codes de caracteres : PowerShell 5.1 lit un .ps1 sans BOM en
      # ANSI, et des guillemets francais ecrits en clair n'y seraient plus ceux du
      # journal (mesure : pas8/essai-2, les titres etaient passes).
      $og = [string][char]0xAB; $fg = [string][char]0xBB
      $motif = $og + ' ([^' + $fg + ']*) ' + $fg
      $rempl = $og + ' (fenetre de l''auteur, titre retire) ' + $fg
      $j = $j | ForEach-Object { [regex]::Replace($_, $motif, { param($m) if (@('Veilleuse', 'Page calme', 'Claude', 'Program Manager', '') -contains $m.Groups[1].Value) { $m.Value } else { $rempl } }) }
      [IO.File]::WriteAllLines($journal, $j)
      $premierPlan = @($j | Where-Object { $_ -like 'presence : premier plan*' })
      foreach ($l in @($premierPlan + @($j | Where-Object { $_ -like 'ilot : ne deploye*' -or $_ -like 'matiere : *' -or $_ -like 'reglages : *' })) | Select-Object -First 6) { Dire ("    journal : " + $l.Substring(0, [Math]::Min(150, $l.Length))) }
      # La matiere lue par le processus doit etre celle que la mesure voulait,
      # et le verre voulu doit etre possible : le troisieme essai a compare
      # opaque a opaque (economiseur en marche) et des lancements y lisaient la
      # matiere d'un autre script (matiere.txt ecrit a tour de role).
      if ($arguments -contains '--ilot-deploye' -and @($j | Where-Object { $_ -like 'ilot : ne deploye*' }).Count -eq 0) {
        $contamine = $true; $pourquoi = 'le drapeau --ilot-deploye n''a pas atteint le processus (pas de « ne deploye » au journal)'
      }
      if ($null -ne $matiere) {
        $lm = @($j | Where-Object { $_ -like 'matiere : *' })
        if ($lm.Count -eq 0) { $contamine = $true; $pourquoi = 'le journal ne dit pas la matiere lue' }
        else {
          $attendu = if ($matiere -eq 'verre') { 'matiere : verre choisi*' } else { 'matiere : galet opaque*' }
          if ($lm[0] -notlike $attendu) { $contamine = $true; $pourquoi = ('la matiere lue n''est pas celle voulue (' + $matiere + ') : ' + $lm[0].Substring(0, [Math]::Min(60, $lm[0].Length))) }
          elseif ($matiere -eq 'verre' -and $lm[0] -like '*verre impossible*') { $contamine = $true; $pourquoi = ('le verre voulu est impossible : ' + $lm[0].Substring($lm[0].IndexOf('impossible'), [Math]::Min(60, $lm[0].Length - $lm[0].IndexOf('impossible')))) }
        }
      }
      # La contamination se lit au journal : la ou l'horloge devait etre
      # devant, un « parait » ; la ou la page calme devait l'etre, un
      # « retire » apres qu'elle est passee, ou pas de « parait » du tout.
      if ($premierPlan.Count -gt 0) {
        if ($pageDevant) {
          $apresPage = $false; $paru = $false
          foreach ($l in $premierPlan) {
            if ($l -like ('*' + $og + ' Page calme ' + $fg + '*')) { $apresPage = $true }
            if ($apresPage -and $l -like '*-> parait*') { $paru = $true }
            if ($apresPage -and $l -like '*-> retire*') { $contamine = $true; $pourquoi = 'une autre fenetre est passee devant la page calme' }
          }
          if (-not $paru) { $contamine = $true; $pourquoi = 'l''ilot n''a pas paru sur la page calme' }
        } else {
          if (@($premierPlan | Where-Object { $_ -like '*-> parait*' }).Count -gt 0) { $contamine = $true; $pourquoi = 'une fenetre est passee devant l''horloge' }
        }
      }
    }
  }
  return [pscustomobject]@{ Cpu = $cpu; Gpu = $gpu; Contamine = $contamine; Pourquoi = $pourquoi; EntreePendant = $entreePendant }
}

$grandeurs = [ordered]@{ Cpu = "processeur, % d'un coeur"; Gpu = 'GPU, mediane du lancement' }
$racine2 = [Math]::Sqrt(2.0)

function Serie($nomSituation, $verreExe, $avantExe, $position, $matV, $matA, $argsV, $argsA, $pageDevant, $attenteGpuSous) {
  $ecarts = @{ Cpu = @(); Gpu = @() }
  $soi = @{ Cpu = @(); Gpu = @() }
  for ($s = 1; $s -le $Series; $s++) {
    $ordre = if ($s % 2) { @('V', 'A', 'A', 'V') } else { @('A', 'V', 'V', 'A') }
    Dire "--- $nomSituation, serie $s : $(($ordre | ForEach-Object { if ($_ -eq 'V') { 'verre' } else { 'avant' } }) -join ', ')"
    $m = @{ V = @(); A = @() }
    foreach ($o in $ordre) {
      if ($o -eq 'V') { $m.V += Mesurer "verre, $nomSituation" $verreExe $position $matV $argsV $pageDevant }
      else { $m.A += Mesurer "avant, $nomSituation" $avantExe $position $matA $argsA $pageDevant }
    }
    foreach ($g in $grandeurs.Keys) {
      $v = @($m.V | ForEach-Object { $_.$g }); $a = @($m.A | ForEach-Object { $_.$g })
      $ecart = ($v[0] + $v[1]) / 2 - ($a[0] + $a[1]) / 2
      $ecarts[$g] += $ecart
      $soi[$g] += [Math]::Abs($v[0] - $v[1]); $soi[$g] += [Math]::Abs($a[0] - $a[1])
      Dire ("    {0} : verre moins avant {1:N2} ; verre contre lui-meme {2:N2}, avant contre lui-meme {3:N2}" -f $grandeurs[$g], $ecart, [Math]::Abs($v[0] - $v[1]), [Math]::Abs($a[0] - $a[1]))
    }
  }
  foreach ($g in $grandeurs.Keys) {
    $seuil = ($soi[$g] | Measure-Object -Maximum).Maximum
    $seuilMoyenne = $seuil / $racine2
    $plusPetit = ($ecarts[$g] | Measure-Object -Minimum).Minimum
    $mediane = Mediane $ecarts[$g]
    $serie = (($ecarts[$g] | ForEach-Object { '{0:N2}' -f $_ }) -join ' ')
    Dire ("  VERDICT {0}, {1} : le verre coute plus que l'avant au-dela du bruit, dans chaque serie : {2}  (verre moins avant par serie : {3} ; mediane {4:N2} ; seuil de l'epreuve, plus grand ecart d'un binaire a lui-meme : {5:N2})" -f $nomSituation, $grandeurs[$g], $(if ($plusPetit -gt $seuil) { 'VRAI' } else { 'FAUX' }), $serie, $mediane, $seuil)
    if ($null -ne $attenteGpuSous -and $g -eq 'Gpu') {
      Dire ("  VERDICT A3 {0}, {1} : la mediane des ecarts tient sous +{2:N2} points : {3}  (mediane {4:N2} ; seuil a l'echelle d'une moyenne, {5:N2} / racine de 2 = {6:N2})" -f $nomSituation, $grandeurs[$g], $attenteGpuSous, $(if ($mediane -lt $attenteGpuSous) { 'VRAI' } else { 'FAUX' }), $mediane, $seuil, $seuilMoyenne)
    } else {
      # « Rien de plus » : le verdict est d'un seul cote, un ecart negatif au-dela
      # du bruit est une economie, pas un cout.
      Dire ("  VERDICT A3 {0}, {1} : la mediane des ecarts ne depasse pas le seuil a l'echelle d'une moyenne ({2:N2} / racine de 2 = {3:N2}) : {4}  (mediane {5:N2}{6})" -f $nomSituation, $grandeurs[$g], $seuil, $seuilMoyenne, $(if ($mediane -le $seuilMoyenne) { 'VRAI' } else { 'FAUX' }), $mediane, $(if ($plusPetit -gt 0) { ' ; ecart positif dans chaque serie, un cout probable' } elseif ($mediane -lt -$seuilMoyenne) { ' ; une economie au-dela du bruit' } else { '' }))
    }
  }
}

$pageProc = $null
$reduit = $false
try {
  if ($Epreuve) {
    # Les deux temoins de la matiere doivent pouvoir tomber : la matiere ecrite
    # a l'envers de la matiere voulue (ce que faisaient trois scripts a la
    # fois), et le verre voulu sous l'economiseur force.
    $a = MesurerUneFois "epreuve, matiere ecrite a l'envers" $Verre '760 30' 'verre' @() $false 'opaque'
    Dire ("  EPREUVE du temoin de la matiere lue : contamine {0}, cause « {1} »" -f $a.Contamine, $a.Pourquoi)
    $b = MesurerUneFois "epreuve, verre sous --batterie=economiseur" $Verre '760 30' 'verre' @('--batterie=economiseur') $false
    Dire ("  EPREUVE du temoin du verre impossible : contamine {0}, cause « {1} »" -f $b.Contamine, $b.Pourquoi)
    $c = MesurerUneFois "epreuve, verre sous --batterie=secteur" $Verre '760 30' 'verre' @('--batterie=secteur') $false
    Dire ("  EPREUVE du forcage secteur : contamine {0}, cause « {1} »" -f $c.Contamine, $c.Pourquoi)
    $d = MesurerUneFois "epreuve, --ilot-deploye" $Verre '760 30' 'opaque' @('--ilot-deploye') $false
    Dire ("  EPREUVE du drapeau --ilot-deploye (doit atteindre le processus) : contamine {0}, cause « {1} »" -f $d.Contamine, $d.Pourquoi)
    $temoinsTombent = $a.Contamine -and $a.Pourquoi -like 'la matiere lue*' -and $b.Contamine -and $b.Pourquoi -like 'le verre voulu est impossible*'
    $forcagesTiennent = -not ($c.Pourquoi -like 'la matiere lue*' -or $c.Pourquoi -like 'le verre voulu*') -and -not ($d.Pourquoi -like 'le drapeau*')
    Dire ("  VERDICT EPREUVE : les deux temoins tombent, le forcage secteur et le drapeau atteignent le processus : {0}" -f $(if ($temoinsTombent -and $forcagesTiennent) { 'VRAI' } else { 'FAUX' }))
  } elseif ($Deploye) {
    if (-not (Test-Path $Page)) { throw "la page calme est requise avec -Deploye : -Page outils/page-calme.py" }
    # L'ecran de l'auteur : ce qui tourne est dit (les noms des processus dont
    # une fenetre est visible, jamais leurs titres), tout est REDUIT — aucune
    # application n'est fermee —, et l'ecran est retabli a la fin.
    $visibles = @(Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -ne '' } | ForEach-Object { $_.ProcessName } | Sort-Object -Unique)
    Dire ("ecran de l'auteur : fenetres visibles des processus " + ($visibles -join ', ') + " ; toutes reduites (MinimizeAll), aucune fermee ; retablies a la fin (UndoMinimizeALL)")
    (New-Object -ComObject Shell.Application).MinimizeAll()
    $reduit = $true
    Start-Sleep -Seconds 3
    # -Situations 2,3 pour ne refaire que celles-la (la 1 acquise, pas 8).
    $voulues = @($Situations -split ',' | ForEach-Object { $_.Trim() })
    if ($voulues -contains '1') {
      Serie "l'horloge devant, ilot retire, galet opaque" $Verre $Avant '760 30' 'opaque' 'opaque' @() @() $false $null
    }
    if ($voulues -contains '2' -or $voulues -contains '3') {
      $pageProc = Start-Process -FilePath 'python' -ArgumentList @($Page, $drapeau, '--duree=5400') -PassThru -RedirectStandardOutput (Join-Path $journaux 'page-calme.txt')
      Start-Sleep -Seconds 2
    }
    # -Secteur : la machine est sur batterie, et l'economiseur du systeme rend
    # le verre impossible (pas 7) ; le binaire integre est force « secteur »
    # des deux cotes, verre comme opaque, et la ligne le dit.
    $force = if ($Secteur) { @('--batterie=secteur') } else { @() }
    if ($voulues -contains '2') {
      Serie "deploye sur la page calme, verre contre opaque" $Verre $Verre '760 30' 'verre' 'opaque' (@('--ilot-deploye') + $force) (@('--ilot-deploye') + $force) $true 3.54
    }
    if ($voulues -contains '3') {
      Serie "replie sur la page calme, verre contre opaque" $Verre $Verre '760 30' 'verre' 'opaque' $force $force $true 3.54
    }
  } else {
    foreach ($place in @(@('place par defaut', '760 30'), @('sur les secondes', '1349 447'))) {
      $nomPlace, $position = $place
      Serie $nomPlace $Verre $Avant $position $null $null @() @() $false $null
    }
  }
  Dire ("lancements : {0}, dont {1} contamine(s) et refait(s) ; alimentation a la fin : {2}" -f $script:lancements, $script:contamines, (Alimentation))
} finally {
  if ($reduit) {
    (New-Object -ComObject Shell.Application).UndoMinimizeALL()
    Dire "ecran de l'auteur retabli (UndoMinimizeALL)"
  }
  if ($null -ne $pageProc) {
    [IO.File]::WriteAllText("$drapeau.fin", "fin`n")
    Start-Sleep -Seconds 1
    Stop-Process -Id $pageProc.Id -Force -ErrorAction SilentlyContinue
    Remove-Item "$drapeau.fin" -ErrorAction SilentlyContinue
    Remove-Item $drapeau -ErrorAction SilentlyContinue
  }
  foreach ($exe in @($Verre, $Avant)) {
    Get-Process | Where-Object { $_.Path -eq $exe } | Stop-Process -Force -ErrorAction SilentlyContinue
  }
  [IO.File]::WriteAllText($pos, $sauvegarde)
  if ($null -ne $matiereAvant) { [IO.File]::WriteAllText($mat, $matiereAvant) } elseif (Test-Path $mat) { Remove-Item $mat }
  $relu = [IO.File]::ReadAllText($pos)
  $matRelu = if (Test-Path $mat) { [IO.File]::ReadAllText($mat) } else { $null }
  Dire ("position restauree : [{0}] ; identique a la sauvegarde : {1} ; matiere restauree : [{2}] ; identique : {3}" -f $relu.Trim(), ($relu -ceq $sauvegarde), $(if ($null -ne $matRelu) { $matRelu.Trim() } else { 'absente' }), ($matRelu -ceq $matiereAvant))
  [IO.File]::WriteAllLines($Sortie, $lignes)
}
