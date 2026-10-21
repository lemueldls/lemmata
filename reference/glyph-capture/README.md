# Glyph capture

Records, for a set of glyphs, the geometry tree a pinned Iosevka build produced and the contours it converted them to. `lemmata-glyphs` reads the result and replays the tree through `lemmata-geom`.

Unlike the other oracles this one patches the build, because the glyph store is built inside `buildFont` and there is no exported entry point that hands out per-glyph geometry.

Apply two edits to a scratch copy of `vendor/Iosevka`:

1. In `packages/font/src/otd-conv/geometry.mjs`, at the top of `convertGlyphGeometryToProxy`, add:

   ```js
   lemmataDumpGlyphs(glyphStore, para);
   ```

2. Append `dump.mjs` to the end of the same file.

Then build with the capture enabled. The set is the one the tests compare
against: every variant `lower-a.ptl` builds, plus the letters, digits and
punctuation the other suites walk. A capture missing one of them fails
`crates/lemmata-glyphs/tests/lower_a.rs` with its name.

```sh
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build
LEMMATA_GLYPH_DUMP=/tmp/opencode/glyphs.bin
LEMMATA_GLYPH_WIDTH=500
LEMMATA_GLYPH_SLOPE=0
LEMMATA_GLYPH_SET='a,a.crossing,a.doubleStoreyFlatBottomSerifed,\
a.doubleStoreyFlatBottomSerifless,\
a.doubleStoreyHookInwardSerifedFlatBottomSerifed,\
a.doubleStoreyHookInwardSerifedFlatBottomSerifless,\
a.doubleStoreyHookInwardSerifedSerifed,\
a.doubleStoreyHookInwardSerifedSerifless,\
a.doubleStoreyHookInwardSerifedTailed,\
a.doubleStoreyHookInwardSerifedToothlessCorner,\
a.doubleStoreyHookInwardSerifedToothlessRounded,a.doubleStoreySerifed,\
a.doubleStoreySerifless,a.doubleStoreyTailed,\
a.doubleStoreyToothlessCorner,a.doubleStoreyToothlessRounded,\
a.singleStoreyDoubleSerifed,a.singleStoreyEarlessCornerSerifed,\
a.singleStoreyEarlessCornerSerifless,a.singleStoreyEarlessCornerTailed,\
a.singleStoreyEarlessRoundedSerifed,\
a.singleStoreyEarlessRoundedSerifless,\
a.singleStoreyEarlessRoundedTailed,a.singleStoreyFlatTopDoubleSerifed,\
a.singleStoreyFlatTopSerifed,a.singleStoreyFlatTopSerifless,\
a.singleStoreyFlatTopTailed,a.singleStoreyFlatTopTailedSerifed,\
a.singleStoreySerifed,a.singleStoreySerifless,a.singleStoreyTailed,\
a.singleStoreyTailedSerifed,a.singleStoreyTopCutSerifed,\
a.singleStoreyTopCutSerifless,a.singleStoreyTopCutTailed,\
aCap.singleStoreyDoubleSerifed,aCap.singleStoreyEarlessCornerSerifed,\
aCap.singleStoreyEarlessCornerSerifless,\
aCap.singleStoreyEarlessCornerTailed,\
aCap.singleStoreyEarlessRoundedSerifed,\
aCap.singleStoreyEarlessRoundedSerifless,\
aCap.singleStoreyEarlessRoundedTailed,\
aCap.singleStoreyFlatTopDoubleSerifed,aCap.singleStoreyFlatTopSerifed,\
aCap.singleStoreyFlatTopSerifless,aCap.singleStoreyFlatTopTailed,\
aCap.singleStoreyFlatTopTailedSerifed,aCap.singleStoreySerifed,\
aCap.singleStoreySerifless,aCap.singleStoreyTailed,\
aCap.singleStoreyTailedSerifed,aCap.singleStoreyTopCutSerifed,\
aCap.singleStoreyTopCutSerifless,aCap.singleStoreyTopCutTailed,\
aHalf.doubleStoreyFlatBottomSerifed,\
aHalf.doubleStoreyFlatBottomSerifless,aHalf.doubleStoreySerifed,\
aHalf.doubleStoreySerifless,aHalf.doubleStoreyTailed,\
aHalf.doubleStoreyToothlessCorner,aHalf.doubleStoreyToothlessRounded,\
ampersand,asciiDoubleQuote,asciiSingleQuote,asterisk,at,b,c,colon,comma,\
d,dollar,e,eight.lnum,equal,exclam,f,five.lnum,four.lnum,g,greater,h,\
hyphen,i,j,k,l,less,m,n,nine.lnum,numberSign,o,one.lnum,p,parenLeft,\
parenRight,percent,period,plus,q,question,r,s,semicolon,seven.lnum,\
six.lnum,slash,space,t,three.lnum,two.lnum,u,v,w,x,y,z,zero.lnum,\
b.serifless,bStroke.serifless,bSlash.serifless,latn/be.serifless,\
bHookTop.serifless,b.motionSerifed,bStroke.motionSerifed,\
bSlash.motionSerifed,latn/be.motionSerifed,b.bottomSerifed,\
bStroke.bottomSerifed,bSlash.bottomSerifed,latn/be.bottomSerifed,\
bHookTop.bottomSerifed,b.serifed,bStroke.serifed,bSlash.serifed,\
latn/be.serifed,b.bottomCutSerifless,bStroke.bottomCutSerifless,\
bSlash.bottomCutSerifless,latn/be.bottomCutSerifless,\
bHookTop.bottomCutSerifless,b.bottomCutMotionSerifed,\
bStroke.bottomCutMotionSerifed,bSlash.bottomCutMotionSerifed,\
latn/be.bottomCutMotionSerifed,b.bottomCutBottomSerifed,\
bStroke.bottomCutBottomSerifed,bSlash.bottomCutBottomSerifed,\
latn/be.bottomCutBottomSerifed,bHookTop.bottomCutBottomSerifed,\
b.bottomCutSerifed,bStroke.bottomCutSerifed,bSlash.bottomCutSerifed,\
latn/be.bottomCutSerifed,b.flatBottomSerifless,\
bStroke.flatBottomSerifless,bSlash.flatBottomSerifless,\
latn/be.flatBottomSerifless,bHookTop.flatBottomSerifless,\
b.flatBottomMotionSerifed,bStroke.flatBottomMotionSerifed,\
bSlash.flatBottomMotionSerifed,latn/be.flatBottomMotionSerifed,\
b.flatBottomBottomSerifed,bStroke.flatBottomBottomSerifed,\
bSlash.flatBottomBottomSerifed,latn/be.flatBottomBottomSerifed,\
bHookTop.flatBottomBottomSerifed,b.flatBottomSerifed,\
bStroke.flatBottomSerifed,bSlash.flatBottomSerifed,\
latn/be.flatBottomSerifed,b.toothlessCornerSerifless,\
bStroke.toothlessCornerSerifless,bSlash.toothlessCornerSerifless,\
latn/be.toothlessCornerSerifless,bHookTop.toothlessCornerSerifless,\
b.toothlessCornerMotionSerifed,bStroke.toothlessCornerMotionSerifed,\
bSlash.toothlessCornerMotionSerifed,\
latn/be.toothlessCornerMotionSerifed,b.toothlessCornerBottomSerifed,\
bStroke.toothlessCornerBottomSerifed,\
bSlash.toothlessCornerBottomSerifed,\
latn/be.toothlessCornerBottomSerifed,\
bHookTop.toothlessCornerBottomSerifed,b.toothlessCornerSerifed,\
bStroke.toothlessCornerSerifed,bSlash.toothlessCornerSerifed,\
latn/be.toothlessCornerSerifed,b.toothlessRoundedSerifless,\
bStroke.toothlessRoundedSerifless,bSlash.toothlessRoundedSerifless,\
latn/be.toothlessRoundedSerifless,bHookTop.toothlessRoundedSerifless,\
b.toothlessRoundedMotionSerifed,bStroke.toothlessRoundedMotionSerifed,\
bSlash.toothlessRoundedMotionSerifed,\
latn/be.toothlessRoundedMotionSerifed,b.toothlessRoundedBottomSerifed,\
bStroke.toothlessRoundedBottomSerifed,\
bSlash.toothlessRoundedBottomSerifed,\
latn/be.toothlessRoundedBottomSerifed,\
bHookTop.toothlessRoundedBottomSerifed,b.toothlessRoundedSerifed,\
bStroke.toothlessRoundedSerifed,bSlash.toothlessRoundedSerifed,\
latn/be.toothlessRoundedSerifed,mathbb/b,d.toothedSerifless,\
dCaron/base.toothedSerifless,dStroke.toothedSerifless,\
latn/de.toothedSerifless,d/phoneticLeft2.toothedSerifless,\
dHookTop.toothedSerifless,d.toothedTopSerifed,\
dCaron/base.toothedTopSerifed,dStroke.toothedTopSerifed,\
latn/de.toothedTopSerifed,d/phoneticLeft2.toothedTopSerifed,\
d.toothedBottomSerifed,dCaron/base.toothedBottomSerifed,\
dStroke.toothedBottomSerifed,latn/de.toothedBottomSerifed,\
dHookTop.toothedBottomSerifed,d.toothedSerifed,\
dCaron/base.toothedSerifed,dStroke.toothedSerifed,\
latn/de.toothedSerifed,d.tailedSerifless,dCaron/base.tailedSerifless,\
dStroke.tailedSerifless,latn/de.tailedSerifless,\
dHookTop.tailedSerifless,d.tailedTopSerifed,\
dCaron/base.tailedTopSerifed,dStroke.tailedTopSerifed,\
latn/de.tailedTopSerifed,d.tailedBottomSerifed,\
dCaron/base.tailedBottomSerifed,dStroke.tailedBottomSerifed,\
latn/de.tailedBottomSerifed,dHookTop.tailedBottomSerifed,\
d.tailedSerifed,dCaron/base.tailedSerifed,dStroke.tailedSerifed,\
latn/de.tailedSerifed,d.flatBottomSerifless,\
dCaron/base.flatBottomSerifless,dStroke.flatBottomSerifless,\
latn/de.flatBottomSerifless,dHookTop.flatBottomSerifless,\
d.flatBottomTopSerifed,dCaron/base.flatBottomTopSerifed,\
dStroke.flatBottomTopSerifed,latn/de.flatBottomTopSerifed,\
d.flatBottomBottomSerifed,dCaron/base.flatBottomBottomSerifed,\
dStroke.flatBottomBottomSerifed,latn/de.flatBottomBottomSerifed,\
dHookTop.flatBottomBottomSerifed,d.flatBottomSerifed,\
dCaron/base.flatBottomSerifed,dStroke.flatBottomSerifed,\
latn/de.flatBottomSerifed,d.toothlessCornerSerifless,\
dCaron/base.toothlessCornerSerifless,dStroke.toothlessCornerSerifless,\
latn/de.toothlessCornerSerifless,dHookTop.toothlessCornerSerifless,\
d.toothlessCornerTopSerifed,dCaron/base.toothlessCornerTopSerifed,\
dStroke.toothlessCornerTopSerifed,latn/de.toothlessCornerTopSerifed,\
d.toothlessCornerBottomSerifed,dCaron/base.toothlessCornerBottomSerifed,\
dStroke.toothlessCornerBottomSerifed,\
latn/de.toothlessCornerBottomSerifed,\
dHookTop.toothlessCornerBottomSerifed,d.toothlessCornerSerifed,\
dCaron/base.toothlessCornerSerifed,dStroke.toothlessCornerSerifed,\
latn/de.toothlessCornerSerifed,d.toothlessCornerRTBSerifless,\
dCaron/base.toothlessCornerRTBSerifless,\
dStroke.toothlessCornerRTBSerifless,latn/de.toothlessCornerRTBSerifless,\
dHookTop.toothlessCornerRTBSerifless,d.toothlessCornerRTBTopSerifed,\
dCaron/base.toothlessCornerRTBTopSerifed,\
dStroke.toothlessCornerRTBTopSerifed,\
latn/de.toothlessCornerRTBTopSerifed,d.toothlessCornerRTBBottomSerifed,\
dCaron/base.toothlessCornerRTBBottomSerifed,\
dStroke.toothlessCornerRTBBottomSerifed,\
latn/de.toothlessCornerRTBBottomSerifed,\
dHookTop.toothlessCornerRTBBottomSerifed,d.toothlessCornerRTBSerifed,\
dCaron/base.toothlessCornerRTBSerifed,dStroke.toothlessCornerRTBSerifed,\
latn/de.toothlessCornerRTBSerifed,d.toothlessRoundedSerifless,\
dCaron/base.toothlessRoundedSerifless,dStroke.toothlessRoundedSerifless,\
latn/de.toothlessRoundedSerifless,dHookTop.toothlessRoundedSerifless,\
d.toothlessRoundedTopSerifed,dCaron/base.toothlessRoundedTopSerifed,\
dStroke.toothlessRoundedTopSerifed,latn/de.toothlessRoundedTopSerifed,\
d.toothlessRoundedBottomSerifed,\
dCaron/base.toothlessRoundedBottomSerifed,\
dStroke.toothlessRoundedBottomSerifed,\
latn/de.toothlessRoundedBottomSerifed,\
dHookTop.toothlessRoundedBottomSerifed,d.toothlessRoundedSerifed,\
dCaron/base.toothlessRoundedSerifed,dStroke.toothlessRoundedSerifed,\
latn/de.toothlessRoundedSerifed,dCurlyTail.toothedSerifless,\
dCurlyTail.toothedTopSerifed,cyrl/djeKomi.toothlessRoundedSerifless,\
cyrl/djeKomi.toothlessRoundedTopSerifed,mathbb/d,F.serifless,\
FBar.serifless,smcpF.serifless,currency/frenchFrancSign.serifless,\
F.topLeftSerifed,FBar.topLeftSerifed,smcpF.topLeftSerifed,\
currency/frenchFrancSign.topLeftSerifed,F.serifed,FBar.serifed,\
smcpF.serifed,currency/frenchFrancSign.serifed,F.serifedCapped,\
FBar.serifedCapped,smcpF.serifedCapped,mathbb/F,I.straight,I.serifless,\
I.serifed,I.shortSerifed,ILonga.straight,ILonga.serifless,\
ILonga.serifed,ILonga.shortSerifed,smcpI,mathbb/I,D.standardSerifless,\
D.standardUnilateralSerifed,D.standardBilateralSerifed,\
D.moreRoundedSerifless,D.moreRoundedUnilateralSerifed,\
D.moreRoundedBilateralSerifed,smcpD.moreRoundedSerifless,\
smcpD.moreRoundedUnilateralSerifed,smcpD.moreRoundedBilateralSerifed,\
Eth,smcpEth,mathbb/D,L.serifless,LSlash.serifless,smcpL.serifless,\
smcpLSlash.serifless,LHighBar.serifless,L.motionSerifed,\
LSlash.motionSerifed,smcpL.motionSerifed,smcpLSlash.motionSerifed,\
LHighBar.motionSerifed,L.serifed,LSlash.serifed,smcpL.serifed,\
smcpLSlash.serifed,LHighBar.serifed,mathbb/L,E.serifless,\
smcpE.serifless,revE.serifless,smcpEBar.serifless,E.topLeftSerifed,\
smcpE.topLeftSerifed,revE.topLeftSerifed,smcpEBar.topLeftSerifed,\
E.serifed,smcpE.serifed,revE.serifed,smcpEBar.serifed,E.serifedCapped,\
smcpE.serifedCapped,revE.serifedCapped,smcpEBar.serifedCapped,mathbb/E,\
p.earedSerifless,pStrokeBottom.earedSerifless,thorn.serifless,\
thornStroke.serifless,thornStrokeBottom.serifless,p.earedMotionSerifed,\
pStrokeBottom.earedMotionSerifed,thorn.motionSerifed,\
thornStroke.motionSerifed,thornStrokeBottom.motionSerifed,\
p.earedBottomSerifed,pStrokeBottom.earedBottomSerifed,\
thorn.bottomSerifed,thornStroke.bottomSerifed,\
thornStrokeBottom.bottomSerifed,p.earedSerifed,\
pStrokeBottom.earedSerifed,thorn.serifed,thornStroke.serifed,\
thornStrokeBottom.serifed,p.flatTopSerifless,\
pStrokeBottom.flatTopSerifless,p.flatTopMotionSerifed,\
pStrokeBottom.flatTopMotionSerifed,p.flatTopBottomSerifed,\
pStrokeBottom.flatTopBottomSerifed,p.flatTopSerifed,\
pStrokeBottom.flatTopSerifed,p.earlessCornerSerifless,\
pStrokeBottom.earlessCornerSerifless,p.earlessCornerMotionSerifed,\
pStrokeBottom.earlessCornerMotionSerifed,p.earlessCornerBottomSerifed,\
pStrokeBottom.earlessCornerBottomSerifed,p.earlessCornerSerifed,\
pStrokeBottom.earlessCornerSerifed,p.earlessRoundedSerifless,\
pStrokeBottom.earlessRoundedSerifless,p.earlessRoundedMotionSerifed,\
pStrokeBottom.earlessRoundedMotionSerifed,p.earlessRoundedBottomSerifed,\
pStrokeBottom.earlessRoundedBottomSerifed,p.earlessRoundedSerifed,\
pStrokeBottom.earlessRoundedSerifed,mathbb/p,h.straightSerifless,\
hStroke.straightSerifless,hHookTop.straightSerifless,h.straightSerifed,\
hStroke.straightSerifed,hHookTop.straightSerifed,\
h.straightMotionSerifed,hStroke.straightMotionSerifed,\
hHookTop.straightMotionSerifed,h.straightTopLeftSerifed,\
hStroke.straightTopLeftSerifed,hHookTop.straightTopLeftSerifed,\
h.tailedSerifless,hStroke.tailedSerifless,hHookTop.tailedSerifless,\
h.tailedSerifed,hStroke.tailedSerifed,hHookTop.tailedSerifed,\
h.tailedMotionSerifed,hStroke.tailedMotionSerifed,\
hHookTop.tailedMotionSerifed,h.tailedTopLeftSerifed,\
hStroke.tailedTopLeftSerifed,hHookTop.tailedTopLeftSerifed,\
heng.serifless,hengStroke.serifless,hengHookTop.serifless,heng.serifed,\
hengStroke.serifed,hengHookTop.serifed,heng.motionSerifed,\
hengStroke.motionSerifed,hengHookTop.motionSerifed,heng.topLeftSerifed,\
hengStroke.topLeftSerifed,hengHookTop.topLeftSerifed,mathbb/h,\
B.standardSerifless,BStroke.standardSerifless,smcpB.standardSerifless,\
smcpBBar.standardSerifless,BBar.standardSerifless,\
B.standardUnilateralSerifed,BStroke.standardUnilateralSerifed,\
smcpB.standardUnilateralSerifed,smcpBBar.standardUnilateralSerifed,\
BBar.standardUnilateralSerifed,B.standardBilateralSerifed,\
BStroke.standardBilateralSerifed,smcpB.standardBilateralSerifed,\
smcpBBar.standardBilateralSerifed,BBar.standardBilateralSerifed,\
B.standardInterruptedSerifless,BStroke.standardInterruptedSerifless,\
smcpB.standardInterruptedSerifless,\
smcpBBar.standardInterruptedSerifless,\
B.standardInterruptedUnilateralSerifed,\
BStroke.standardInterruptedUnilateralSerifed,\
smcpB.standardInterruptedUnilateralSerifed,\
smcpBBar.standardInterruptedUnilateralSerifed,\
B.standardInterruptedBilateralSerifed,\
BStroke.standardInterruptedBilateralSerifed,\
smcpB.standardInterruptedBilateralSerifed,\
smcpBBar.standardInterruptedBilateralSerifed,B.moreAsymmetricSerifless,\
BStroke.moreAsymmetricSerifless,BBar.moreAsymmetricSerifless,\
B.moreAsymmetricUnilateralSerifed,\
BStroke.moreAsymmetricUnilateralSerifed,\
BBar.moreAsymmetricUnilateralSerifed,B.moreAsymmetricBilateralSerifed,\
BStroke.moreAsymmetricBilateralSerifed,\
BBar.moreAsymmetricBilateralSerifed,\
B.moreAsymmetricInterruptedSerifless,\
BStroke.moreAsymmetricInterruptedSerifless,\
B.moreAsymmetricInterruptedUnilateralSerifed,\
BStroke.moreAsymmetricInterruptedUnilateralSerifed,\
B.moreAsymmetricInterruptedBilateralSerifed,\
BStroke.moreAsymmetricInterruptedBilateralSerifed,n.straightSerifless,\
n.straightTopLeftSerifed,n.straightMotionSerifed,n.straightSerifed,\
n.tailedSerifless,n.tailedTopLeftSerifed,n.tailedMotionSerifed,\
n.tailedSerifed,n.flatTopStraightSerifless,\
n.flatTopStraightTopLeftSerifed,n.flatTopStraightMotionSerifed,\
n.flatTopStraightSerifed,n.flatTopTailedSerifless,\
n.flatTopTailedTopLeftSerifed,n.flatTopTailedMotionSerifed,\
n.flatTopTailedSerifed,n.earlessCornerStraightSerifless,\
n.earlessCornerStraightTopLeftSerifed,\
n.earlessCornerStraightMotionSerifed,n.earlessCornerStraightSerifed,\
n.earlessCornerTailedSerifless,n.earlessCornerTailedTopLeftSerifed,\
n.earlessCornerTailedMotionSerifed,n.earlessCornerTailedSerifed,\
n.earlessRoundedStraightSerifless,\
n.earlessRoundedStraightTopLeftSerifed,\
n.earlessRoundedStraightMotionSerifed,n.earlessRoundedStraightSerifed,\
n.earlessRoundedTailedSerifless,n.earlessRoundedTailedTopLeftSerifed,\
n.earlessRoundedTailedMotionSerifed,n.earlessRoundedTailedSerifed,\
Q.straight,Q.crossing,Q.detachedTailed,Q.detachedBendTailed,\
smcpQ.straight,smcpQ.crossing,smcpQ.detachedTailed,\
smcpQ.detachedBendTailed,O,smcpO,OSlash,oSlash,OBar,oBar,mathbb/O,\
mathbb/o,H.serifless,H.topLeftSerifed,H.topLeftBottomRightSerifed,\
H.serifed,H.serifedBGR,H.tailedSerifless,H.tailedTopLeftSerifed,\
H.tailedSerifed,H.tailedSerifedBGR,H.serifedExceptBottomRight,\
smcpH.serifless,smcpH.topLeftSerifed,smcpH.topLeftBottomRightSerifed,\
smcpH.serifed,smcpH.serifedBGR,smcpH.tailedSerifless,\
smcpH.tailedTopLeftSerifed,smcpH.tailedSerifed,smcpH.tailedSerifedBGR,\
smcpH.serifedExceptBottomRight,P.closedSerifless,P.closedMotionSerifed,\
P.closedSerifed,P.openSerifless,P.openMotionSerifed,P.openSerifed,\
smcpP.closedSerifless,smcpP.closedMotionSerifed,smcpP.closedSerifed,\
smcpP.openSerifless,smcpP.openMotionSerifed,smcpP.openSerifed,\
N.standard,N.standardSerifless,N.standardMotionSerifed,\
N.standardSerifed,N.asymmetric,N.asymmetricSerifless,\
N.asymmetricMotionSerifed,N.asymmetricSerifed,smcpN.standard,\
smcpN.standardSerifless,smcpN.standardMotionSerifed,\
smcpN.standardSerifed,smcpN.asymmetric,smcpN.asymmetricSerifless,\
smcpN.asymmetricMotionSerifed,smcpN.asymmetricSerifed,\
dotlessi.serifless,dotlessi.serifed,dotlessi.hookyBottom,\
dotlessiCap.serifless,dotlessiCap.serifed,dotlessiCap.hookyBottom,\
l.serifless,l.serifed,l.hookyBottom,cyrl/palochka.serifless,\
cyrl/palochka.serifed,cyrl/palochka.hookyBottom,M.hangingSerifless,\
M.hangingMotionSerifed,M.hangingSmallCyrl,M.hangingSerifed,\
M.flatBottomSerifless,M.flatBottomMotionSerifed,M.flatBottomSmallCyrl,\
M.flatBottomSerifed,M.slantedSidesHangingSerifless,\
M.slantedSidesHangingMotionSerifed,M.slantedSidesHangingSmallCyrl,\
M.slantedSidesHangingSerifed,M.slantedSidesFlatBottomSerifless,\
M.slantedSidesFlatBottomMotionSerifed,M.slantedSidesFlatBottomSmallCyrl,\
M.slantedSidesFlatBottomSerifed,M.grekCapitalSan,M.grekSmallSan,\
smcpM.hangingSerifless,smcpM.hangingMotionSerifed,\
smcpM.hangingSmallCyrl,smcpM.hangingSerifed,smcpM.flatBottomSerifless,\
smcpM.flatBottomMotionSerifed,smcpM.flatBottomSmallCyrl,\
smcpM.flatBottomSerifed,smcpM.slantedSidesHangingSerifless,\
smcpM.slantedSidesHangingMotionSerifed,\
smcpM.slantedSidesHangingSmallCyrl,smcpM.slantedSidesHangingSerifed,\
smcpM.slantedSidesFlatBottomSerifless,\
smcpM.slantedSidesFlatBottomMotionSerifed,\
smcpM.slantedSidesFlatBottomSmallCyrl,\
smcpM.slantedSidesFlatBottomSerifed,smcpM.grekCapitalSan,\
smcpM.grekSmallSan,m.serifless,m.serifed,dotAbove.serifless,tittleAbove,\
dotAbove.round,dotAbove.square,TieMark{tittleAbove},meng.serifless,\
meng.serifed,m.topLeftSerifed,meng.topLeftSerifed,m.bottomRightSerifed,\
meng.bottomRightSerifed,m.topLeftAndBottomRightSerifed,\
meng.topLeftAndBottomRightSerifed,m.tailedSerifless,m.tailedSerifed,\
m.tailedTopLeftSerifed,m.tailedBottomRightSerifed,\
m.tailedTopLeftAndBottomRightSerifed,m.shortLegSerifless,\
meng.shortLegSerifless,m.shortLegSerifed,meng.shortLegSerifed,\
m.shortLegTopLeftSerifed,meng.shortLegTopLeftSerifed,\
m.shortLegBottomRightSerifed,meng.shortLegBottomRightSerifed,\
m.shortLegTopLeftAndBottomRightSerifed,\
meng.shortLegTopLeftAndBottomRightSerifed,m.shortLegTailedSerifless,\
m.shortLegTailedSerifed,m.shortLegTailedTopLeftSerifed,\
m.shortLegTailedBottomRightSerifed,\
m.shortLegTailedTopLeftAndBottomRightSerifed,m.flatTopSerifless,\
meng.flatTopSerifless,m.flatTopSerifed,meng.flatTopSerifed,\
m.flatTopTopLeftSerifed,meng.flatTopTopLeftSerifed,\
m.flatTopBottomRightSerifed,meng.flatTopBottomRightSerifed,\
m.flatTopTopLeftAndBottomRightSerifed,\
meng.flatTopTopLeftAndBottomRightSerifed,m.flatTopTailedSerifless,\
m.flatTopTailedSerifed,m.flatTopTailedTopLeftSerifed,\
m.flatTopTailedBottomRightSerifed,\
m.flatTopTailedTopLeftAndBottomRightSerifed,m.flatTopShortLegSerifless,\
meng.flatTopShortLegSerifless,m.flatTopShortLegSerifed,\
meng.flatTopShortLegSerifed,m.flatTopShortLegTopLeftSerifed,\
meng.flatTopShortLegTopLeftSerifed,m.flatTopShortLegBottomRightSerifed,\
meng.flatTopShortLegBottomRightSerifed,\
m.flatTopShortLegTopLeftAndBottomRightSerifed,\
meng.flatTopShortLegTopLeftAndBottomRightSerifed,\
m.flatTopShortLegTailedSerifless,m.flatTopShortLegTailedSerifed,\
m.flatTopShortLegTailedTopLeftSerifed,\
m.flatTopShortLegTailedBottomRightSerifed,\
m.flatTopShortLegTailedTopLeftAndBottomRightSerifed,\
TieMark{i.TRK/tittleAbove},f.serifless,f.serifed,f.extended,f.tailed,\
f.diagonalTailed,f.seriflessCrossbarAtXHeight,\
f.serifedCrossbarAtXHeight,f.extendedCrossbarAtXHeight,\
f.tailedCrossbarAtXHeight,f.diagonalTailedCrossbarAtXHeight,\
f.seriflessCrossbarAtHalfAscenderHeight,\
f.serifedCrossbarAtHalfAscenderHeight,\
f.extendedCrossbarAtHalfAscenderHeight,\
f.tailedCrossbarAtHalfAscenderHeight,\
f.diagonalTailedCrossbarAtHalfAscenderHeight,f.narrowSerifless,\
f.narrowSerifed,f.narrowExtended,f.narrowTailed,f.narrowDiagonalTailed,\
f.narrowSeriflessCrossbarAtXHeight,f.narrowSerifedCrossbarAtXHeight,\
f.narrowExtendedCrossbarAtXHeight,f.narrowTailedCrossbarAtXHeight,\
f.narrowDiagonalTailedCrossbarAtXHeight,\
f.narrowSeriflessCrossbarAtHalfAscenderHeight,\
f.narrowSerifedCrossbarAtHalfAscenderHeight,\
f.narrowExtendedCrossbarAtHalfAscenderHeight,\
f.narrowTailedCrossbarAtHalfAscenderHeight,\
f.narrowDiagonalTailedCrossbarAtHalfAscenderHeight,f.flatHookSerifless,\
f.flatHookSerifed,f.flatHookExtended,f.flatHookTailed,\
f.flatHookDiagonalTailed,f.flatHookSeriflessCrossbarAtXHeight,\
f.flatHookSerifedCrossbarAtXHeight,f.flatHookExtendedCrossbarAtXHeight,\
f.flatHookTailedCrossbarAtXHeight,\
f.flatHookDiagonalTailedCrossbarAtXHeight,\
f.flatHookSeriflessCrossbarAtHalfAscenderHeight,\
f.flatHookSerifedCrossbarAtHalfAscenderHeight,\
f.flatHookExtendedCrossbarAtHalfAscenderHeight,\
f.flatHookTailedCrossbarAtHalfAscenderHeight,\
f.flatHookDiagonalTailedCrossbarAtHalfAscenderHeight'
npm run build -- ttf-unhinted::RefA
```

`LEMMATA_GLYPH_SLOPE=0` is not optional. A plan builds four faces and each one
runs the dump, so without it the italic lands on top of the upright and every
shape check downstream still passes. `LEMMATA_GLYPH_WIDTH` narrows the face, but
the italic has the same width and gets through.

```sh
```

A third capture carries the letters and every spelling of each, which is what
`crates/lemmata-cli/tests/combinations.rs` walks:

```
LEMMATA_GLYPH_DUMP=target/variants.bin LEMMATA_GLYPH_WIDTH=500 \
LEMMATA_GLYPH_SET=$(python3 -c "print(','.join(n for n in open('target/glyphs.bin.names') if n.split('.')[0] in set('abcdefghijklmnopqrstuvwxyz') and ('.' not in n or n.split('.')[1].isalnum())))") \
npm run build -- ttf-unhinted::RefA
```

It lands 567 glyphs, and only the binary is kept: the combinations test reads the
trees and the contours, not the stage side files. It is separate from the coverage
capture for the reason `docs/math.md` gives, that the outline tests are
calibrated against that one.

A second capture of the same set at `spacing = "quasi-proportional"` is what
gives the site a proportional face. The spacing is a property of the drawings,
so it is a second build rather than a second set of advances:
`web/public/glyphs-quasi.bin` is the site's set drawn that way, and
`docs/plan.md` records what changes.

The plan must hold only the upright slope. `reference/private-build-plans.toml`
gives RefA an italic slope too, both are 500 units wide, and both pass the width
guard, so the dump lands on whichever the build writes last. Build a plan with
the italic slope removed.

`LEMMATA_GLYPH_DUMP` is the binary output path, `LEMMATA_GLYPH_SET` selects by glyph name and `LEMMATA_GLYPH_CODEPOINTS` by codepoint. `LEMMATA_GLYPH_WIDTH` pins the advance width, because a plan can emit more than one font and the dump otherwise lands on whichever is written last.

A cached build does not re-run the dump. Changing the plan back and forth to capture one slope at a time is exactly the case that hits this: the second build finds its output already in `dist/` and skips the work the dump is hooked into, so the capture keeps its old contents and looks like a build that succeeded. Remove `dist/<plan>` to force it.

The width pin is not enough on its own. A plan with two slopes at the same width, an upright and an italic, emits two fonts that both pass the guard, and the dump lands on the last one written. Capturing a particular slope means building a plan that holds only that slope: it is worth doing, because the failure is silent and produces a capture that looks correct until you compare it with the other one.

The capture writes `<path>` (the trees), `<path>.json` (the contours Iosevka converted them to), `<path>.tree-arcs.json` (the tree's arcs, before overlap resolution), `<path>.arcs.json` (after it), `<path>.expansions.json` (each di-spiro node's expanded sides and the raw arcs the solver produced from them) and `<path>.fairized.json` (the fairized shape the quadifier consumes). Copy them to `target/` for `cargo test -p lemmata-glyphs` and `cargo test -p lemmata-cli`.

## Binary layout

Little-endian throughout. A `u32` glyph count, then per glyph:

- name: `u32` length and UTF-8 bytes
- advance width: `f64`
- unicodes: `u32` count, then `u32` each
- glyph gizmo: six `f64` (`xx, xy, yx, yy, tx, ty`)
- geometry node

A node starts with a `u8` tag:

| Tag | Kind | Fields |
| --- | --- | --- |
| 0 | invalid | none |
| 1 | contours | `u32` contour count; per contour `u32` arc count; per arc four `f64` pairs |
| 2 | spiro | gizmo, `u8` closed, knots |
| 3 | di-spiro | gizmo, `f64` contrast, `u8` closed, bi-knots |
| 4 | spiro pen | gizmo, `u8` closed, profile points, pen knots |
| 5 | stroke | gizmo, `f64` radius, `f64` contrast, `u8` inside, node |
| 6 | remove holes | gizmo, node |
| 7 | simplify | gizmo, node |
| 8 | combine | `u32` part count, nodes |
| 9 | boolean | operator string, `u32` operand count, nodes |
| 10 | transformed | gizmo, node |
| 11 | tagged | node |
| 12 | radical | node |
| 13 | reference | node |

A knot is a type string, its `f64` coordinates, and flags; see `dump.mjs` for the exact fields.
