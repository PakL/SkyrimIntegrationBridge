{
	Export a list of every form of interest
}
unit UserScript;

var
	sl: TStringList;
	filename: String;

function Initialize: integer;
begin
	sl := TStringList.Create;
end;

function ExtractMagicEffectsDescription(e: IInterface): String;
var
	spelleffects, spellme: IwbElement;
	spelleffectmessage, spellmag, spelldur, spelltemp: String;
	i: Integer;
begin
	spelleffects := ElementByPath(e, 'Effects');
	spelleffectmessage := '';
	for i := 0 to ElementCount(spelleffects)-1 do begin
		spellme := ElementByIndex(spelleffects, i);
		spellmag := FloatToStr(GetElementNativeValues(spellme, 'EFIT\Magnitude'));
		spelldur := FloatToStr(GetElementNativeValues(spellme, 'EFIT\Duration'));
		spelltemp := GetElementEditValues(LinksTo(ElementByPath(spellme, 'EFID')), 'DNAM');
		spelltemp := StringReplace(spelltemp, '<mag>', spellmag, [rfReplaceAll, rfIgnoreCase]);
		spelltemp := StringReplace(spelltemp, '<dur>', spelldur, [rfReplaceAll, rfIgnoreCase]);
		spelltemp := StringReplace(spelltemp, #13, '', [rfReplaceAll, rfIgnoreCase]);
		spelltemp := StringReplace(spelltemp, #10, '', [rfReplaceAll, rfIgnoreCase]);
		if spelleffectmessage <> '' then
			spelleffectmessage := ' ' + spelleffectmessage + spelltemp
		else
			spelleffectmessage := spelltemp
	end;
	Result := Trim(spelleffectmessage);
end;

function GetNPCFlags(e: IInterface): String;
var
	flaglist: TStringList;
	i: Integer;
begin
	Result := '';
	flaglist := TStringList.Create;
	flaglist.Text := FlagValues(ElementByPath(e, 'ACBS\Flags'));
	for i := 0 to Pred(flaglist.Count) do
		if GetElementEditValues(e, 'ACBS\Flags\' + flaglist[i]) = '1' then begin
			if Result <> '' then
				Result := Result + ', ' + flaglist[i]
			else
				Result := flaglist[i];
		end;
	flaglist.Free;
end;

function GetNPCSkills(e: IInterface): String;
var
	skilllist: IwbElement;
	skillnames: Array[0..17] of String;
	i, curr, first, firstsk, second, secondsk, third, thirdsk: Integer;
begin
	skillnames[0] := 'One Handed';
	skillnames[1] := 'Two Handed';
	skillnames[2] := 'Archery';
	skillnames[3] := 'Block';
	skillnames[4] := 'Smithing';
	skillnames[5] := 'Heavy Armor';
	skillnames[6] := 'Light Armor';
	skillnames[7] := 'Pickpocket';
	skillnames[8] := 'Lockpicking';
	skillnames[9] := 'Sneak';
	skillnames[10] := 'Alchemy';
	skillnames[11] := 'Speechcraft';
	skillnames[12] := 'Alteration';
	skillnames[13] := 'Conjuration';
	skillnames[14] := 'Destruction';
	skillnames[15] := 'Illusion';
	skillnames[16] := 'Restoration';
	skillnames[17] := 'Enchanting';

	skilllist := ElementByPath(e, 'DNAM\Skill Values');
	first := 0;
	second := 0;
	third := 0;
	firstsk := 0;
	secondsk := 0;
	thirdsk := 0;
	curr := 0;

	for i := 0 to ElementCount(skilllist)-1 do begin
		curr := StrToInt(GetElementEditValues(e, 'DNAM\Skill Values\' + Name(ElementByIndex(skilllist, i))));
		if curr > firstsk then begin
			thirdsk := secondsk;
			third := second;
			secondsk := firstsk;
			second := first;
			firstsk := curr;
			first := i;
			end
		else if curr > secondsk then begin
			thirdsk := secondsk;
			third := second;
			secondsk := curr;
			second := i;
			end
		else if curr > thirdsk then begin
			thirdsk := curr;
			third := i;
			end
		;
	end;
	Result := skillnames[first] + ', ' + skillnames[second] + ', ' + skillnames[third];
end;

function Process(e: IInterface): integer;
var
	sig, filesig, npclevel, npcrace, spelltemp, formid, fs: String;
begin
	sig := Signature(e);
	filename := BaseName(GetFile(e));
	filesig := BaseName(GetFile(e)) + ';' + sig + ';';
	
	if not IsMaster(e) then
		Exit;
		
	formid := Copy(IntToHex(GetLoadOrderFormID(e), 8), 3, 6);
	if GetIsESL(GetFile(e)) then
		formid := Copy(formid, 4, 3);
	
	e := WinningOverride(e);
	
	if sig= 'NPC_' then begin
		npclevel := '';
		if GetElementEditValues(e, 'ACBS\Flags\PC Level Mult') = '1' then
			npclevel := GetElementEditValues(e, 'ACBS\Level Mult') + ',' + GetElementEditValues(e, 'ACBS\Calc min level') + ',' + GetElementEditValues(e, 'ACBS\Calc max level')
		else
			npclevel := GetElementEditValues(e, 'ACBS\Level');
		
		npcrace := GetElementEditValues(LinksTo(ElementByPath(e, 'RNAM')), 'FULL');
		if npcrace = '' then npcrace := GetElementEditValues(LinksTo(ElementByPath(e, 'RNAM')), 'EDID');
		
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + npcrace + ';' + npclevel + ';' + GetNPCSkills(e) + ';' + GetNPCFlags(e) + ';');
		end
	else if sig = 'ALCH' then
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + GetElementEditValues(e, 'DATA') + ';' + IntToStr(GetElementNativeValues(e, 'ENIT\Value')) + ';' + ExtractMagicEffectsDescription(e) + ';;')
	else if sig = 'AMMO' then
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + GetElementEditValues(e, 'DATA\Damage') + ';' + GetElementEditValues(e, 'DATA\Value') + ';' + GetElementEditValues(e, 'DATA\Weight') + ';;')
	else if sig = 'ARMO' then
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + GetElementEditValues(e, 'BOD2\Armor Type') + ';' + GetElementEditValues(e, 'DNAM') + ';' + GetElementEditValues(e, 'DATA\Value') + ';' + GetElementEditValues(e, 'DATA\Weight') + ';')
	else if sig = 'WEAP' then begin
		spelltemp := ExtractMagicEffectsDescription(LinksTo(ElementByPath(e, 'EITM')));
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + GetElementEditValues(e, 'DNAM\Skill') + ';' + GetElementEditValues(e, 'DATA\Damage') + ';' + GetElementEditValues(e, 'DATA\Value') + ';' + GetElementEditValues(e, 'DATA\Weight') + ';' + spelltemp);
		end
	else if sig = 'INGR' then
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + GetElementEditValues(e, 'DATA\Value') + ';' + GetElementEditValues(e, 'DATA\Weight') + ';;;')
	else if sig = 'MISC' then
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + GetElementEditValues(e, 'DATA\Value') + ';' + GetElementEditValues(e, 'DATA\Weight') + ';;;')
	else if sig = 'SPEL' then
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + GetElementEditValues(e, 'SPIT\Type') + ';' + GetElementEditValues(e, 'SPIT\Base Cost') + ';' + ExtractMagicEffectsDescription(e) + ';;')
	else if sig = 'SCRL' then
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + ExtractMagicEffectsDescription(e) + ';;;;')
	else if sig = 'ENCH' then
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + GetElementEditValues(e, 'ENIT\Target Type') + ';' + ExtractMagicEffectsDescription(e) + ';;;')
	else if sig = 'MGEF' then begin
		spelltemp := GetElementEditValues(e, 'DNAM');
		spelltemp := StringReplace(spelltemp, #13, '', [rfReplaceAll, rfIgnoreCase]);
		spelltemp := StringReplace(spelltemp, #10, '', [rfReplaceAll, rfIgnoreCase]);
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + spelltemp + ';;;;');
		end
	else if sig = 'BOOK' then begin
		spelltemp := GetElementEditValues(e, 'DATA\Skill');
		if GetElementEditValues(e, 'DATA\Spell') <> '' then
			spelltemp := ExtractMagicEffectsDescription(LinksTo(ElementByPath(e, 'DATA\Spell')));
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';' + GetElementEditValues(e, 'DATA\Value') + ';' + GetElementEditValues(e, 'DATA\Weight') + ';' + spelltemp + ';;');
		end
	{else if GetElementEditValues(e, 'EDID') <> '' then
		sl.Add(filesig + formid + ';' + GetElementEditValues(e, 'EDID') + ';' + GetElementEditValues(e, 'FULL') + ';;;;;')}
	;
end;

function Finalize: integer;
var
	fname: string;
begin
	fname := ProgramPath + 'Edit Scripts\' + filename + '.csv';
	AddMessage('Saving list to ' + fname);
	sl.SaveToFile(fname);
	sl.Free;
end;

end.