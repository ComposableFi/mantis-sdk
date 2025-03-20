/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/escrow.json`.
 */
export type Escrow = {
  "address": "54TyyV9KbeYamLmFFppLE1qZsi2rZgncrtRrU5y7DiTM",
  "metadata": {
    "name": "escrow",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "cancelIntent",
      "discriminator": [
        67,
        73,
        238,
        244,
        208,
        89,
        225,
        59
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "escrowPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              }
            ]
          }
        },
        {
          "name": "intentPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  105,
                  110,
                  116,
                  101,
                  110,
                  116
                ]
              },
              {
                "kind": "arg",
                "path": "intentId"
              }
            ]
          }
        },
        {
          "name": "tokenInMint"
        },
        {
          "name": "feeTokenInPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ]
          }
        },
        {
          "name": "feeSolPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "userTokenInAta",
          "writable": true
        },
        {
          "name": "srcUser",
          "writable": true
        },
        {
          "name": "escrowTokenInAta",
          "writable": true
        },
        {
          "name": "clock",
          "address": "SysvarC1ock11111111111111111111111111111111"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "token2022Program",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "intentId",
          "type": "string"
        }
      ]
    },
    {
      "name": "collectFees",
      "discriminator": [
        164,
        152,
        207,
        99,
        30,
        186,
        19,
        182
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true,
          "relations": [
            "escrowPda"
          ]
        },
        {
          "name": "tokenInMint"
        },
        {
          "name": "escrowPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              }
            ]
          }
        },
        {
          "name": "feeTokenInPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ]
          }
        },
        {
          "name": "feeSolPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "escrowTokenInAta",
          "writable": true
        },
        {
          "name": "feeRecipientAta",
          "writable": true
        },
        {
          "name": "feeRecipient",
          "writable": true
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "token2022Program",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "nativeSol",
          "type": "bool"
        }
      ]
    },
    {
      "name": "escrowFunds",
      "discriminator": [
        177,
        157,
        138,
        132,
        217,
        63,
        234,
        128
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "escrowPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              }
            ]
          }
        },
        {
          "name": "tokenInMint"
        },
        {
          "name": "userTokenInAta",
          "writable": true
        },
        {
          "name": "escrowTokenInAta",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "escrowPda"
              },
              {
                "kind": "const",
                "value": [
                  6,
                  221,
                  246,
                  225,
                  215,
                  101,
                  161,
                  147,
                  217,
                  203,
                  225,
                  70,
                  206,
                  235,
                  121,
                  172,
                  28,
                  180,
                  133,
                  237,
                  95,
                  91,
                  55,
                  145,
                  58,
                  140,
                  245,
                  133,
                  126,
                  255,
                  0,
                  169
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "feeTokenInPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ]
          }
        },
        {
          "name": "feeSolPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "intentPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  105,
                  110,
                  116,
                  101,
                  110,
                  116
                ]
              },
              {
                "kind": "arg",
                "path": "new_intent.intent_id"
              }
            ]
          }
        },
        {
          "name": "feesPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "clock",
          "address": "SysvarC1ock11111111111111111111111111111111"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "token2022Program",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "newIntent",
          "type": {
            "defined": {
              "name": "newIntent"
            }
          }
        }
      ]
    },
    {
      "name": "finalizeAuction",
      "discriminator": [
        220,
        209,
        175,
        193,
        57,
        132,
        241,
        168
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true,
          "relations": [
            "escrowPda"
          ]
        },
        {
          "name": "escrowPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              }
            ]
          }
        },
        {
          "name": "intentPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  105,
                  110,
                  116,
                  101,
                  110,
                  116
                ]
              },
              {
                "kind": "arg",
                "path": "intentId"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "intentId",
          "type": "string"
        },
        {
          "name": "solver",
          "type": "pubkey"
        },
        {
          "name": "amountOut",
          "type": "string"
        }
      ]
    },
    {
      "name": "initialize",
      "discriminator": [
        175,
        175,
        109,
        31,
        13,
        152,
        155,
        237
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "escrowPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              }
            ]
          }
        },
        {
          "name": "feesPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "feeSolPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "setFees",
      "discriminator": [
        137,
        178,
        49,
        58,
        0,
        245,
        242,
        190
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true,
          "relations": [
            "escrowPda"
          ]
        },
        {
          "name": "escrowPda",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              }
            ]
          }
        },
        {
          "name": "feesPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  115
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "defaultFlatFee",
          "type": "u64"
        },
        {
          "name": "agentFlatFee",
          "type": "u64"
        },
        {
          "name": "defaultPercentFee",
          "type": "u16"
        },
        {
          "name": "agentPercentFee",
          "type": "u16"
        }
      ]
    },
    {
      "name": "solveIntentLocal",
      "discriminator": [
        117,
        194,
        126,
        219,
        56,
        115,
        37,
        112
      ],
      "accounts": [
        {
          "name": "solver",
          "writable": true,
          "signer": true
        },
        {
          "name": "intentPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  105,
                  110,
                  116,
                  101,
                  110,
                  116
                ]
              },
              {
                "kind": "arg",
                "path": "intentId"
              }
            ]
          }
        },
        {
          "name": "escrowPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              }
            ]
          }
        },
        {
          "name": "tokenInMint"
        },
        {
          "name": "tokenOutMint"
        },
        {
          "name": "escrowTokenInAta",
          "writable": true
        },
        {
          "name": "solverTokenInAta",
          "writable": true
        },
        {
          "name": "solverTokenOutAta",
          "writable": true
        },
        {
          "name": "userTokenOutAta",
          "writable": true
        },
        {
          "name": "dstUser",
          "writable": true
        },
        {
          "name": "feeSolPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "token2022Program",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "intentId",
          "type": "string"
        }
      ]
    },
    {
      "name": "solveIntentRemote",
      "discriminator": [
        222,
        3,
        223,
        105,
        201,
        123,
        213,
        25
      ],
      "accounts": [
        {
          "name": "solver",
          "writable": true,
          "signer": true
        },
        {
          "name": "escrowPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              }
            ]
          }
        },
        {
          "name": "tokenOutMint"
        },
        {
          "name": "solverTokenOutAta",
          "writable": true
        },
        {
          "name": "userTokenOutAta",
          "writable": true
        },
        {
          "name": "dstUser",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "token2022Program",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "intentId",
          "type": "string"
        },
        {
          "name": "srcChainId",
          "type": "u8"
        },
        {
          "name": "dstUser",
          "type": "pubkey"
        },
        {
          "name": "tokenOut",
          "type": "pubkey"
        },
        {
          "name": "amountOut",
          "type": "u64"
        }
      ]
    },
    {
      "name": "unlockSolverFunds",
      "discriminator": [
        208,
        27,
        171,
        61,
        158,
        164,
        228,
        1
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true,
          "relations": [
            "escrowPda"
          ]
        },
        {
          "name": "escrowPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              }
            ]
          }
        },
        {
          "name": "tokenInMint"
        },
        {
          "name": "escrowTokenInAta",
          "writable": true
        },
        {
          "name": "solverTokenInAta",
          "writable": true
        },
        {
          "name": "solver",
          "writable": true
        },
        {
          "name": "feeSolPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "intentPda",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  105,
                  110,
                  116,
                  101,
                  110,
                  116
                ]
              },
              {
                "kind": "arg",
                "path": "intentId"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "token2022Program",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "intentId",
          "type": "string"
        },
        {
          "name": "srcChainId",
          "type": "u8"
        },
        {
          "name": "dstUser",
          "type": "string"
        },
        {
          "name": "tokenOut",
          "type": "string"
        },
        {
          "name": "amountOut",
          "type": "string"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "emptyAccount",
      "discriminator": [
        174,
        156,
        186,
        113,
        230,
        158,
        33,
        215
      ]
    },
    {
      "name": "escrowAccount",
      "discriminator": [
        36,
        69,
        48,
        18,
        128,
        225,
        125,
        135
      ]
    },
    {
      "name": "feesAccount",
      "discriminator": [
        57,
        211,
        239,
        69,
        152,
        254,
        46,
        122
      ]
    },
    {
      "name": "intent",
      "discriminator": [
        247,
        162,
        35,
        165,
        254,
        111,
        129,
        109
      ]
    }
  ],
  "events": [
    {
      "name": "canceledIntent",
      "discriminator": [
        129,
        42,
        89,
        207,
        195,
        173,
        63,
        214
      ]
    },
    {
      "name": "collectedFees",
      "discriminator": [
        36,
        15,
        20,
        38,
        181,
        135,
        83,
        60
      ]
    },
    {
      "name": "escrowedFunds",
      "discriminator": [
        184,
        181,
        119,
        172,
        40,
        179,
        29,
        19
      ]
    },
    {
      "name": "finalizedAuction",
      "discriminator": [
        241,
        76,
        181,
        186,
        25,
        193,
        164,
        75
      ]
    },
    {
      "name": "solvedIntentLocal",
      "discriminator": [
        124,
        127,
        139,
        9,
        75,
        28,
        79,
        60
      ]
    },
    {
      "name": "solvedIntentRemote",
      "discriminator": [
        10,
        205,
        97,
        121,
        17,
        41,
        109,
        172
      ]
    },
    {
      "name": "unlockedSolverFunds",
      "discriminator": [
        159,
        69,
        73,
        123,
        253,
        157,
        115,
        69
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "invalidTimeout",
      "msg": "Invalid intent timeout"
    },
    {
      "code": 6001,
      "name": "intentNotTimedOut",
      "msg": "Intent has not timed out yet"
    },
    {
      "code": 6002,
      "name": "badPublicKey",
      "msg": "Unable to parse Pubkey from String"
    },
    {
      "code": 6003,
      "name": "intentAlreadyExists",
      "msg": "Intent already exists"
    },
    {
      "code": 6004,
      "name": "srcUserNotSender",
      "msg": "Intent src_user does not match the sender"
    },
    {
      "code": 6005,
      "name": "intentDoesNotExist",
      "msg": "Intent does not exist"
    },
    {
      "code": 6006,
      "name": "srcUserMismatch",
      "msg": "Intent src_user is different than the provided src_user"
    },
    {
      "code": 6007,
      "name": "srcChainIdMismatch",
      "msg": "Intent src_chain_id is different than the provided src_chain_id"
    },
    {
      "code": 6008,
      "name": "invalidRemoteChainId",
      "msg": "Remote chain_id is the same as the local chain_id"
    },
    {
      "code": 6009,
      "name": "dstUserMismatch",
      "msg": "Intent dst_user is different than the provided dst_user"
    },
    {
      "code": 6010,
      "name": "invalidTokenOut",
      "msg": "Invalid token_out"
    },
    {
      "code": 6011,
      "name": "invalidAmountOut",
      "msg": "Invalid amount_out"
    },
    {
      "code": 6012,
      "name": "tokenInNotAtaMint",
      "msg": "Intent token_in does not match the ATA mint"
    },
    {
      "code": 6013,
      "name": "srcUserNotAtaOwner",
      "msg": "Intent src_user does not match the ATA owner"
    }
  ],
  "types": [
    {
      "name": "canceledIntent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "intentId",
            "type": "string"
          }
        ]
      }
    },
    {
      "name": "collectedFees",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIn",
            "type": "pubkey"
          },
          {
            "name": "feeRecipient",
            "type": "pubkey"
          },
          {
            "name": "feeAmount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "emptyAccount",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "escrowAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "escrowedFunds",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "intentId",
            "type": "string"
          }
        ]
      }
    },
    {
      "name": "feesAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "defaultFlatFee",
            "type": "u64"
          },
          {
            "name": "agentFlatFee",
            "type": "u64"
          },
          {
            "name": "defaultPercentFee",
            "type": "u16"
          },
          {
            "name": "agentPercentFee",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "finalizedAuction",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "intentId",
            "type": "string"
          },
          {
            "name": "solver",
            "type": "pubkey"
          },
          {
            "name": "amountOut",
            "type": "string"
          }
        ]
      }
    },
    {
      "name": "intent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "intentId",
            "type": "string"
          },
          {
            "name": "srcUser",
            "type": "pubkey"
          },
          {
            "name": "dstUser",
            "type": "string"
          },
          {
            "name": "srcChainId",
            "type": "u8"
          },
          {
            "name": "dstChainId",
            "type": "u8"
          },
          {
            "name": "tokenIn",
            "type": "pubkey"
          },
          {
            "name": "amountIn",
            "type": "u64"
          },
          {
            "name": "tokenOut",
            "type": "string"
          },
          {
            "name": "amountOut",
            "type": "string"
          },
          {
            "name": "solver",
            "type": "pubkey"
          },
          {
            "name": "creation",
            "type": "u64"
          },
          {
            "name": "timeout",
            "type": "u64"
          },
          {
            "name": "aiAgent",
            "type": "bool"
          },
          {
            "name": "refundAmount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "newIntent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "intentId",
            "type": "string"
          },
          {
            "name": "srcUser",
            "type": "pubkey"
          },
          {
            "name": "dstUser",
            "type": "string"
          },
          {
            "name": "dstChainId",
            "type": "u8"
          },
          {
            "name": "tokenIn",
            "type": "pubkey"
          },
          {
            "name": "amountIn",
            "type": "u64"
          },
          {
            "name": "tokenOut",
            "type": "string"
          },
          {
            "name": "amountOut",
            "type": "string"
          },
          {
            "name": "timeout",
            "type": "u64"
          },
          {
            "name": "aiAgent",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "solvedIntentLocal",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "intentId",
            "type": "string"
          },
          {
            "name": "solver",
            "type": "pubkey"
          },
          {
            "name": "dstUser",
            "type": "pubkey"
          },
          {
            "name": "tokenIn",
            "type": "pubkey"
          },
          {
            "name": "amountIn",
            "type": "u64"
          },
          {
            "name": "tokenOut",
            "type": "pubkey"
          },
          {
            "name": "amountOut",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "solvedIntentRemote",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "intentId",
            "type": "string"
          },
          {
            "name": "dstUser",
            "type": "pubkey"
          },
          {
            "name": "tokenOut",
            "type": "pubkey"
          },
          {
            "name": "amountOut",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "unlockedSolverFunds",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "intentId",
            "type": "string"
          },
          {
            "name": "solver",
            "type": "pubkey"
          },
          {
            "name": "tokenIn",
            "type": "pubkey"
          },
          {
            "name": "amountIn",
            "type": "u64"
          }
        ]
      }
    }
  ]
};
