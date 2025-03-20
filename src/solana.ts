import { Program, AnchorProvider, setProvider } from "@coral-xyz/anchor";
import { useAnchorWallet, useConnection } from "@solana/wallet-adapter-react";
import type { Escrow } from "./solana/escrow.ts";
import idl from "../idls/escrow.json";
 
const { connection } = useConnection();
const wallet = useAnchorWallet();
 
const provider = new AnchorProvider(connection, wallet, {});
setProvider(provider);

export const program = new Program(idl as Escrow, {
  connection,
});
