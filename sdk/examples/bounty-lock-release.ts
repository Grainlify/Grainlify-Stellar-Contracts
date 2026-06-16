import { Keypair } from '@stellar/stellar-sdk';
import { BountyEscrowClient } from '../src/bounty-escrow-client';

async function main() {
  // Configuration
  const config = {
    contractId: 'CBTG2...', // Replace with actual contract ID
    rpcUrl: 'https://soroban-testnet.stellar.org',
    networkPassphrase: 'Test SDF Network ; September 2015',
  };

  const client = new BountyEscrowClient(config);

  // Generate some keypairs for demonstration
  const adminKeypair = Keypair.random();
  const depositorKeypair = Keypair.random();
  const contributorKeypair = Keypair.random();
  const tokenAddress = 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC'; // XLM on testnet

  console.log('1. Initializing Bounty Escrow Contract');
  try {
    await client.init(adminKeypair.publicKey(), tokenAddress, adminKeypair);
    console.log('Successfully initialized contract');
  } catch (error) {
    console.log('Initialization mock catch:', error.message);
  }

  console.log('\n2. Locking Funds for a Bounty');
  const bountyId = 1n;
  const amount = 10000000n; // 100 XLM
  const deadline = Math.floor(Date.now() / 1000) + 86400 * 7; // 7 days from now
  
  try {
    await client.lockFunds(
      depositorKeypair.publicKey(),
      bountyId,
      amount,
      deadline,
      depositorKeypair
    );
    console.log('Funds locked successfully');
  } catch (error) {
    console.log('Lock funds mock catch:', error.message);
  }

  console.log('\n3. Partial Release of Funds');
  const partialAmount = 2500000n; // 25 XLM
  try {
    await client.partialRelease(
      bountyId,
      contributorKeypair.publicKey(),
      partialAmount,
      adminKeypair // admin or authorized caller
    );
    console.log('Partial release successful');
  } catch (error) {
    console.log('Partial release mock catch:', error.message);
  }

  console.log('\n4. Full Release of Remaining Funds');
  try {
    await client.releaseFunds(
      bountyId,
      contributorKeypair.publicKey(),
      adminKeypair
    );
    console.log('Full release successful');
  } catch (error) {
    console.log('Full release mock catch:', error.message);
  }

  console.log('\n5. Query Escrow Info');
  try {
    const info = await client.getEscrowInfo(bountyId);
    console.log('Escrow Info:', info);
  } catch (error) {
    console.log('Query mock catch:', error.message);
  }
}

main().catch(console.error);
