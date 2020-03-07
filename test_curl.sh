#!/bin/bash

# curl 'http://dipbt.bundestag.de/dip21.web/searchProcedures/advanced_search_list.do' \
#      -H 'User-Agent: Some agent' \
#      -H 'Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8' \
#      -H 'Accept-Language: en-US,en;q=0.5' \
#      --compressed \
#      -H 'Referer: http://dipbt.bundestag.de/dip21.web/searchProcedures/advanced_search_list.do' \
#      -H 'Content-Type: application/x-www-form-urlencoded' \
#      -H 'Origin: http://dipbt.bundestag.de' \
#      -H 'DNT: 1' \
#      -H 'Connection: keep-alive' \
#      -H 'Cookie: SESSIONID=9D76311D91FB135EF7ADF492B405C052.dip21; JSESSIONID=9D76311D91FB135EF7ADF492B405C052.dip21; JSESSIONID=7DB1C42E819F8FC4CD04EC9EEC9D6400.dip21; SESSIONID=7DB1C42E819F8FC4CD04EC9EEC9D6400.dip21' \
#      -H 'Upgrade-Insecure-Requests: 1' \
#      -H 'Pragma: no-cache' \
#      -H 'Cache-Control: no-cache' \
#      --data 'drsId=&plprId=&aeDrsId=&aePlprId=&vorgangId=&procedureContext=&vpId=&formChanged=false&promptUser=false&overrideChanged=true&javascriptActive=yes&personId=&personNachname=&prompt=no&anchor=urheber&wahlperiodeaktualisiert=false&wahlperiode=8&startDatum=&endDatum=&includeVorgangstyp=UND&nummer=&suchwort=&suchwortUndSchlagwort=ODER&schlagwort1=&linkSchlagwort2=UND&schlagwort2=&linkSchlagwort3=UND&schlagwort3=&unterbegriffsTiefe=0&sachgebiet=&includeKu=UND&ressort=&nachname=&vorname=&verkuendungsblatt=BGBl+I&jahrgang=2019&heftnummer=&seite=54&verkuendungStartDatum=&verkuendungEndDatum=&btBrBeteiligung=alle&gestaOrdnungsnummer=&beratungsstand=&signaturParlamentsarchiv=&method=Suchen' \

curl 'http://dipbt.bundestag.de/extrakt/ba/WP19/2553/255353.html'
