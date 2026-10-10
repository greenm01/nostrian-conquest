import sys, json
sys.dont_write_bytecode = True
sys.path.insert(0, __import__('os').path.dirname(__import__('os').path.abspath(__file__)))
import nc_combat_analysis as m

base_as=list(m.AS); base_ds=list(m.DS); base_cost=list(m.COST)
variants=[('baseline',{}, {}, {}),
          ('DD DS7',{}, {0:7}, {}),
          ('DD DS8',{}, {0:8}, {}),
          ('DD DS10',{}, {0:10}, {}),
          ('DD AS12 DS8',{0:12}, {0:8}, {}),
          ('DD AS12 DS8 cost6',{0:12}, {0:8}, {0:6}),
          ('CA DS35',{}, {1:35}, {}),
          ('BB DS120',{}, {2:120}, {}),
          ('SB DS150',{}, {3:150}, {})]
for label,changes_a,changes_d,changes_c in variants:
    m.AS[:]=base_as; m.DS[:]=base_ds; m.COST[:]=base_cost
    for i,v in changes_a.items(): m.AS[i]=v
    for i,v in changes_d.items(): m.DS[i]=v
    for i,v in changes_c.items(): m.COST[i]=v
    dd150=150//m.COST[0]
    cases=[('DD vs CA equal150',m.force(dd=dd150),m.force(ca=10)),
           ('CA vs BB equal450',m.force(ca=30),m.force(bb=10)),
           ('10 DD vs 1 SB',m.force(dd=10),m.force(sb=1))]
    if label.startswith('CA') or label.startswith('BB'): cases=cases[1:2]
    elif label.startswith('SB'): cases=cases[2:]
    elif label!='baseline': cases=[cases[0]]
    for case,a,b in cases:
        for mode in ('C','X'):
            result=m.run(case,a,b,mode,N=6000)
            result['variant']=label
            print(json.dumps(result),flush=True)
