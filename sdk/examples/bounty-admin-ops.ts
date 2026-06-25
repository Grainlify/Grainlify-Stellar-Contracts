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
  const newAdminKeypair = Keypair.random();
  const validGAddress = 'GAXN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG';

  console.log('1. Updating Fee Configuration');
  try {
    // Set 1% fee (100 bps) and 2% penalty (200 bps) to a specific fee collector
    await client.updateFeeConfig(
      100n, // 1% flat fee
      200n, // 2% penalty fee
      validGAddress, // Fee collector address
      true, // Fees enabled
      adminKeypair
    );
    console.log('Successfully updated fee configuration.');
  } catch (error: any) {
    console.log('Update fee config error/mock catch:', error.message);
  }

  console.log('\n2. Managing Pausability (Global & Specific Flags)');
  try {
    // Globally pause contract operations in emergency
    await client.setEmergencyPause(true, adminKeypair);
    console.log('Emergency pause activated globally.');

    // Adjust specific pause flags
    await client.setPaused(
      false, // global_paused = false (resume overall operations)
      true,  // lock_paused = true (prevent new locks)
      false, // release_paused = false (allow releases)
      adminKeypair
    );
    console.log('Fine-grained pause configuration updated.');
  } catch (error: any) {
    console.log('Pausability management error/mock catch:', error.message);
  }

  console.log('\n3. Configuring Circuit Breaker (Safety Limits)');
  try {
    // Define circuit breaker thresholds: 5 failures to open, 3 successes to close, 10 errors max log size
    await client.setCircuitBreakerConfig(5, 3, 10, adminKeypair);
    console.log('Circuit breaker safety limits defined.');

    // Query current status
    const status = await client.getCircuitBreakerStatus();
    console.log('Circuit Breaker Status:', status);
  } catch (error: any) {
    console.log('Circuit breaker configuration error/mock catch:', error.message);
  }

  console.log('\n4. Setting Anti-Abuse Administrator');
  try {
    await client.setAntiAbuseAdmin(newAdminKeypair.publicKey(), adminKeypair);
    console.log('Anti-abuse administrator address updated.');

    const antiAbuseAdmin = await client.getAntiAbuseAdmin();
    console.log('Current Anti-Abuse Administrator:', antiAbuseAdmin);
  } catch (error: any) {
    console.log('Anti-abuse configuration error/mock catch:', error.message);
  }
}

main().catch(console.error);
