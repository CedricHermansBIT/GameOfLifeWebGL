const int R = 9;
const int kernel_size = 20;
const int kernel_length = 400;

const float T = 10.0;

const float m = 0.15;
const float s = 0.015;


const float kernel[kernel_length] = float[kernel_length](
    // Kernel 3: smooth circle
    0.0,0.0,0.0,0.0,0.0,0.000045616297728074831150778951,0.000096671729943841451472216764,0.000162139589331421852614126267,0.000219274815057191784168147408,0.000242128830969278829692015176,0.000219274815057191784168147408,0.000162139589331421852614126267,0.000096671729943841451472216764,0.000045616297728074831150778951,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.000107350016903296342600515612,0.000267161131757292439736078959,0.000520048830124939984696774697,0.000815068961015209532232350664,0.001054849971816461352489002756,0.001147138086817652263132982782,0.001054849971816461352489002756,0.000815068961015209532232350664,0.000520048830124939984696774697,0.000267161131757292439736078959,0.000107350016903296342600515612,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.000040877091782617437437045288,0.000162139589331421852614126267,0.000474045766410290136429195318,0.001054849971816461352489002756,0.001852925999995098070571852844,0.002669680662768485722435851670,0.003267703942930182993420462623,0.003484697819526596032635445965,0.003267703942930182993420462623,0.002669680662768485722435851670,0.001852925999995098070571852844,0.001054849971816461352489002756,0.000474045766410290136429195318,0.000162139589331421852614126267,0.000040877091782617437437045288,0.0,0.0,0.0,0.0,0.0,0.000162139589331421852614126267,0.000570008216434549639176077029,0.001465607013365864035384089625,0.002860028085071759235136212851,0.004431927162683423591926779039,0.005739620852256533539703653446,0.006532191618362248250462265275,0.006787264565880952657539459949,0.006532191618362248250462265275,0.005739620852256533539703653446,0.004431927162683423591926779039,0.002860028085071759235136212851,0.001465607013365864035384089625,0.000570008216434549639176077029,0.000162139589331421852614126267,0.0,0.0,0.0,0.0,0.000107350016903296342600515612,0.000474045766410290136429195318,0.001465607013365864035384089625,0.003267703942930182993420462623,0.005472680461102287161057056153,0.007270535784077095629862608206,0.008201385678671447437837471739,0.008457819192909443728467522305,0.008476267676715620175142973380,0.008457819192909443728467522305,0.008201385678671447437837471739,0.007270535784077095629862608206,0.005472680461102287161057056153,0.003267703942930182993420462623,0.001465607013365864035384089625,0.000474045766410290136429195318,0.000107350016903296342600515612,0.0,0.0,0.000045616297728074831150778951,0.000267161131757292439736078959,0.001054849971816461352489002756,0.002860028085071759235136212851,0.005472680461102287161057056153,0.007701312546226767898061016382,0.008476267676715620175142973380,0.007967335497873210409247590746,0.007144864400305562038284712401,0.006787264565880952657539459949,0.007144864400305562038284712401,0.007967335497873210409247590746,0.008476267676715620175142973380,0.007701312546226767898061016382,0.005472680461102287161057056153,0.002860028085071759235136212851,0.001054849971816461352489002756,0.000267161131757292439736078959,0.000045616297728074831150778951,0.0,0.000096671729943841451472216764,0.000520048830124939984696774697,0.001852925999995098070571852844,0.004431927162683423591926779039,0.007270535784077095629862608206,0.008476267676715620175142973380,0.007461862744640526852457629303,0.005502276654146399763323227461,0.004001927547559624119555277133,0.003484697819526593864231100994,0.004001927547559624119555277133,0.005502276654146399763323227461,0.007461862744640526852457629303,0.008476267676715620175142973380,0.007270535784077095629862608206,0.004431927162683423591926779039,0.001852925999995098070571852844,0.000520048830124939984696774697,0.000096671729943841451472216764,0.0,0.000162139589331421852614126267,0.000815068961015209532232350664,0.002669680662768485722435851670,0.005739620852256533539703653446,0.008201385678671447437837471739,0.007967335497873210409247590746,0.005502276654146399763323227461,0.002972283455571383178200894903,0.001552153782923017776018692615,0.001147138086817653564175589764,0.001552153782923017776018692615,0.002972283455571383178200894903,0.005502276654146399763323227461,0.007967335497873210409247590746,0.008201385678671447437837471739,0.005739620852256533539703653446,0.002669680662768485722435851670,0.000815068961015209532232350664,0.000162139589331421852614126267,0.0,0.000219274815057191784168147408,0.001054849971816461352489002756,0.003267703942930182993420462623,0.006532191618362248250462265275,0.008457819192909443728467522305,0.007144864400305562038284712401,0.004001927547559624119555277133,0.001552153782923017776018692615,0.000486736205923236344914051266,0.000242128830969278829692015176,0.000486736205923236344914051266,0.001552153782923017776018692615,0.004001927547559624119555277133,0.007144864400305562038284712401,0.008457819192909443728467522305,0.006532191618362248250462265275,0.003267703942930182993420462623,0.001054849971816461352489002756,0.000219274815057191784168147408,0.0,0.000242128830969278829692015176,0.001147138086817652263132982782,0.003484697819526596032635445965,0.006787264565880952657539459949,0.008476267676715620175142973380,0.006787264565880952657539459949,0.003484697819526593864231100994,0.001147138086817653564175589764,0.000242128830969278829692015176,0.000032768573918977272321686328,0.000242128830969278829692015176,0.001147138086817653564175589764,0.003484697819526593864231100994,0.006787264565880952657539459949,0.008476267676715620175142973380,0.006787264565880952657539459949,0.003484697819526596032635445965,0.001147138086817652263132982782,0.000242128830969278829692015176,0.0,0.000219274815057191784168147408,0.001054849971816461352489002756,0.003267703942930182993420462623,0.006532191618362248250462265275,0.008457819192909443728467522305,0.007144864400305562038284712401,0.004001927547559624119555277133,0.001552153782923017776018692615,0.000486736205923236344914051266,0.000242128830969278829692015176,0.000486736205923236344914051266,0.001552153782923017776018692615,0.004001927547559624119555277133,0.007144864400305562038284712401,0.008457819192909443728467522305,0.006532191618362248250462265275,0.003267703942930182993420462623,0.001054849971816461352489002756,0.000219274815057191784168147408,0.0,0.000162139589331421852614126267,0.000815068961015209532232350664,0.002669680662768485722435851670,0.005739620852256533539703653446,0.008201385678671447437837471739,0.007967335497873210409247590746,0.005502276654146399763323227461,0.002972283455571383178200894903,0.001552153782923017776018692615,0.001147138086817653564175589764,0.001552153782923017776018692615,0.002972283455571383178200894903,0.005502276654146399763323227461,0.007967335497873210409247590746,0.008201385678671447437837471739,0.005739620852256533539703653446,0.002669680662768485722435851670,0.000815068961015209532232350664,0.000162139589331421852614126267,0.0,0.000096671729943841451472216764,0.000520048830124939984696774697,0.001852925999995098070571852844,0.004431927162683423591926779039,0.007270535784077095629862608206,0.008476267676715620175142973380,0.007461862744640526852457629303,0.005502276654146399763323227461,0.004001927547559624119555277133,0.003484697819526593864231100994,0.004001927547559624119555277133,0.005502276654146399763323227461,0.007461862744640526852457629303,0.008476267676715620175142973380,0.007270535784077095629862608206,0.004431927162683423591926779039,0.001852925999995098070571852844,0.000520048830124939984696774697,0.000096671729943841451472216764,0.0,0.000045616297728074831150778951,0.000267161131757292439736078959,0.001054849971816461352489002756,0.002860028085071759235136212851,0.005472680461102287161057056153,0.007701312546226767898061016382,0.008476267676715620175142973380,0.007967335497873210409247590746,0.007144864400305562038284712401,0.006787264565880952657539459949,0.007144864400305562038284712401,0.007967335497873210409247590746,0.008476267676715620175142973380,0.007701312546226767898061016382,0.005472680461102287161057056153,0.002860028085071759235136212851,0.001054849971816461352489002756,0.000267161131757292439736078959,0.000045616297728074831150778951,0.0,0.0,0.000107350016903296342600515612,0.000474045766410290136429195318,0.001465607013365864035384089625,0.003267703942930182993420462623,0.005472680461102287161057056153,0.007270535784077095629862608206,0.008201385678671447437837471739,0.008457819192909443728467522305,0.008476267676715620175142973380,0.008457819192909443728467522305,0.008201385678671447437837471739,0.007270535784077095629862608206,0.005472680461102287161057056153,0.003267703942930182993420462623,0.001465607013365864035384089625,0.000474045766410290136429195318,0.000107350016903296342600515612,0.0,0.0,0.0,0.0,0.000162139589331421852614126267,0.000570008216434549639176077029,0.001465607013365864035384089625,0.002860028085071759235136212851,0.004431927162683423591926779039,0.005739620852256533539703653446,0.006532191618362248250462265275,0.006787264565880952657539459949,0.006532191618362248250462265275,0.005739620852256533539703653446,0.004431927162683423591926779039,0.002860028085071759235136212851,0.001465607013365864035384089625,0.000570008216434549639176077029,0.000162139589331421852614126267,0.0,0.0,0.0,0.0,0.0,0.000040877091782617437437045288,0.000162139589331421852614126267,0.000474045766410290136429195318,0.001054849971816461352489002756,0.001852925999995098070571852844,0.002669680662768485722435851670,0.003267703942930182993420462623,0.003484697819526596032635445965,0.003267703942930182993420462623,0.002669680662768485722435851670,0.001852925999995098070571852844,0.001054849971816461352489002756,0.000474045766410290136429195318,0.000162139589331421852614126267,0.000040877091782617437437045288,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.000107350016903296342600515612,0.000267161131757292439736078959,0.000520048830124939984696774697,0.000815068961015209532232350664,0.001054849971816461352489002756,0.001147138086817652263132982782,0.001054849971816461352489002756,0.000815068961015209532232350664,0.000520048830124939984696774697,0.000267161131757292439736078959,0.000107350016903296342600515612,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.000045616297728074831150778951,0.000096671729943841451472216764,0.000162139589331421852614126267,0.000219274815057191784168147408,0.000242128830969278829692015176,0.000219274815057191784168147408,0.000162139589331421852614126267,0.000096671729943841451472216764,0.000045616297728074831150778951,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0
);
