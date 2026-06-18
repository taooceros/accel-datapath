#import "../../template.typ": *
#import "support.typ": *
#import "fragments/slide01.typ": slide_01
#import "fragments/slide02.typ": slide_02
#import "fragments/slide03.typ": slide_03
#import "fragments/slide04.typ": slide_04
#import "fragments/slide05.typ": slide_05
#import "fragments/slide06.typ": slide_06
#import "fragments/slide07.typ": slide_07
#import "fragments/slide08.typ": slide_08
#import "fragments/slide09.typ": slide_09
#import "fragments/slide10.typ": slide_10

#deck(
  margin: (x: 42pt, y: 28pt),
  size: 13pt,
  leading: 0.88em,
  spacing: 0.5em,
  footer: none,
  footer-right: none,
)[
  #set page(fill: bg-fill)

  #slide_01()
  #slide_02()
  #slide_03()
  #slide_04()
  #slide_05()
  #slide_06()
  #slide_07()
  #slide_08()
  #slide_09()
  #slide_10()
]
