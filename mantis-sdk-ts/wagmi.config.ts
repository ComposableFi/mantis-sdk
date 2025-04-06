import { defineConfig } from '@wagmi/cli'
import EscrowAbi from './abis/escrow.json'
import { Abi } from 'viem'; 


export default defineConfig({
  out: 'src/ethereum/escrow.ts',
  contracts: [
    {
      name: 'Escrow',
      abi: EscrowAbi as Abi,
      address: '0xaf55771e9cd32f93532670ef358c8703d598505c'
    }
  ],
  plugins: [],
})
