// The equation corpus. Every construct a math font has to serve: a variable,
// arithmetic, a script, a fraction, a radical, a large operator with limits,
// and a stretchy delimiter pair.
//
// The harness in crates/lemmata-cli/tests/typst.rs compiles this against the
// built font and fails when a renderer falls back to a font of its own.

#set text(font: "Lemmata", size: 12pt)
#show math.equation: set text(font: "Lemmata")

$ x + y = z $

$ 1 + 2 - 3 times 4 $

$ x^2 + 2x + 1 = (x + 1)^2 $

$ a/b = c/d $

$ sqrt(x^2 + y^2) $

$ sum_(i=1)^n i = (n(n+1))/2 $

$ integral_0^1 x^2 dif x = 1/3 $

$ (a + b)/c <= sqrt(x^2 + y^2) $

$ lim_(x -> 0) sin(x)/x = 1 $

$ alpha + beta = gamma $

// These four need a delimiter or a radical taller than its base glyph, which is
// what the size variants and the assemblies are for.

$ sqrt(1/2) $

$ (1/2) $

$ [1/2] $

$ {1/2} $
