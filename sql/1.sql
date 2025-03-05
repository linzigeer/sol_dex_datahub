create table pools (
  addr varchar(128) primary key,
  dex varchar(64) not null,
  mint_a varchar(128) not null,
  mint_b varchar(128) not null,
  decimals_a tinyint unsigned not null,
  decimals_b tinyint unsigned not null,
  created_at datetime not null default now()
);


create table trades (
    blk_ts datetime not null,
    slot bigint unsigned not null,
    txid varchar(256) not null,
    idx bigint unsigned not null,
    mint varchar(128) not null,
    decimals tinyint unsigned not null,
    trader varchar(128) not null,
    dex varchar(64) not null,
    pool varchar(128) not null,
    is_buy boolean not null,
    sol_amt bigint unsigned not null,
    token_amt bigint unsigned not null,
    price_sol double not null,
    created_at datetime not null default now(),
    constraint uni_idx_txid_idx unique (txid,idx)
);
