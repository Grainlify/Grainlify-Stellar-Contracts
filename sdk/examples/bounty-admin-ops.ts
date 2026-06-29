import { Keypair } from '@stellar/stellar-sdk';
import { BountyEscrowClient } from '../src/bounty-escrow-client';

export async function runAdminOpsExample(
  client: BountyEscrowClient,
  adminKeypair: Keypair,
  newAdminKeypair: Keypair,
  signer1: Keypair,
  signer2: Keypair
) {
  console.log('--- Bounty Escrow Admin Operations Example ---');

  console.log('\n1. Updating Fee Configuration');
  try {
    // Set lock fee to 1.5% (150 bps) and release fee to 1.0% (100 bps)
    await client.updateFeeConfig(
      150n,
      100n,
      adminKeypair.publicKey(),
      true,
      adminKeypair
    );
    console.log('Fee config update routed successfully');
  } catch (error: any) {
    console.log('updateFeeConfig catch:', error.message);
  }

  console.log('\n2. Toggling Pause Flags');
  try {
    // Pause lock operations while keeping release and refund active
    await client.setPaused(true, false, false, adminKeypair);
    console.log('Pause flags update routed successfully');
  } catch (error: any) {
    console.log('setPaused catch:', error.message);
  }

  console.log('\n3. Configuring Amount Policy (Min/Max Lock Limits)');
  try {
    // Require locked amounts to be between 10 XLM (100,000,000 stroops) and 1000 XLM
    await client.setAmountPolicy(
      adminKeypair.publicKey(),
      100_000_000n,
      10_000_000_000n,
      adminKeypair
    );
    console.log('Amount policy set successfully');
  } catch (error: any) {
    console.log('setAmountPolicy catch:', error.message);
  }

  console.log('\n4. Setting Governance Contract Address and Version requirements');
  try {
    const govAddress = 'GAVK6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG';
    await client.setGovernanceContract(govAddress, adminKeypair);
    await client.setMinGovernanceVersion(2, adminKeypair);
    console.log('Governance settings applied successfully');
  } catch (error: any) {
    console.log('Governance configuration catch:', error.message);
  }

  console.log('\n5. Configuring and Resetting Circuit Breaker');
  try {
    // Set circuit breaker admin
    await client.setCircuitBreakerAdmin(newAdminKeypair.publicKey(), adminKeypair);

    // Set configuration: open circuit after 3 consecutive failures, reset after 5 successes
    await client.setCircuitBreakerConfig(3, 5, 20, adminKeypair);

    // Reset circuit breaker state (calls require CB admin auth)
    await client.resetCircuit(newAdminKeypair.publicKey(), newAdminKeypair);
    console.log('Circuit breaker setup completed successfully');
  } catch (error: any) {
    console.log('Circuit breaker configuration catch:', error.message);
  }

  console.log('\n6. Updating Multisig Configuration & Approving Large Releases');
  try {
    // Configure multisig: releases above 500 XLM require 2 of 2 signatures
    await client.updateMultisigConfig(
      5_000_000_000n,
      [signer1.publicKey(), signer2.publicKey()],
      2,
      adminKeypair
    );

    // Signers approve a release on bounty #123 for contributor
    const contributor = Keypair.random().publicKey();
    await client.approveLargeRelease(123n, contributor, signer1.publicKey(), signer1);
    await client.approveLargeRelease(123n, contributor, signer2.publicKey(), signer2);
    console.log('Multisig config and approvals submitted successfully');
  } catch (error: any) {
    console.log('Multisig admin actions catch:', error.message);
  }

  console.log('\n7. Setting Anti-Abuse Configuration');
  try {
    // Define an anti-abuse admin and whitelist an address
    const whitelistAddress = Keypair.random().publicKey();
    await client.setAntiAbuseAdmin(newAdminKeypair.publicKey(), adminKeypair);
    await client.setWhitelist(whitelistAddress, true, adminKeypair);
    console.log('Anti-abuse configuration executed successfully');
  } catch (error: any) {
    console.log('Anti-abuse configuration catch:', error.message);
  }

  console.log('\n8. Querying Admin Views');
  try {
    const view = await client.getAdminAuditView();
    console.log('Audit view snapshot:', view);
  } catch (error: any) {
    console.log('getAdminAuditView catch:', error.message);
  }
}

if (require.main === module) {
  const config = {
    contractId: 'CBTG2M4XXWNDH7GCHXZT6E2I3J644MFRZQK6CUKL4WJY6WQZXY3P2M6L',
    rpcUrl: 'https://soroban-testnet.stellar.org',
    networkPassphrase: 'Test SDF Network ; September 2015',
  };
  const client = new BountyEscrowClient(config);
  runAdminOpsExample(
    client,
    Keypair.random(),
    Keypair.random(),
    Keypair.random(),
    Keypair.random()
  ).catch(console.error);
}
