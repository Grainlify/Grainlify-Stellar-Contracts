import { nativeToScVal } from '@stellar/stellar-sdk';
import { decodeContractEvent } from '../src/index';

async function main() {
  console.log('--- Soroban Event Decoding Example ---');

  // Example 1: Decoding FundsLocked event from bounty_escrow
  console.log('\n1. Decoding a Bounty Escrow FundsLocked event:');
  
  // Simulated raw topics and value from RPC or transaction meta
  const bountyId = 42n;
  const mockBountyTopics = [
    nativeToScVal('f_lock'), // topic 0: Event symbol name
    nativeToScVal(bountyId), // topic 1: bounty ID
  ];
  
  const mockBountyValue = nativeToScVal({
    version: 2,
    bounty_id: bountyId,
    amount: 50000000n, // 500 tokens
    depositor: 'GAXN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG',
    deadline: Math.floor(Date.now() / 1000) + 86400 * 7, // 7 days from now
  });

  try {
    const event = decodeContractEvent(mockBountyTopics, mockBountyValue);
    
    // Demonstrate discriminated union properties matching
    if (event.type === 'f_lock') {
      console.log('Successfully decoded FundsLockedEvent!');
      console.log(`- Bounty ID: ${event.bounty_id}`);
      console.log(`- Depositor: ${event.depositor}`);
      console.log(`- Amount: ${event.amount} (smallest units)`);
      console.log(`- Deadline: ${new Date(Number(event.deadline) * 1000).toISOString()}`);
    }
  } catch (error: any) {
    console.error('Decoding failed:', error.message);
  }

  // Example 2: Decoding Payout event from program_escrow
  console.log('\n2. Decoding a Program Escrow Payout event:');
  
  const mockProgramTopics = [
    nativeToScVal('Payout'),
  ];
  
  const mockProgramValue = nativeToScVal({
    version: 2,
    program_id: 'campaign-2026',
    recipient: 'GBZN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG',
    amount: 150000000n, // 1500 tokens
    remaining_balance: 350000000n, // 3500 tokens remaining
  });

  try {
    const event = decodeContractEvent(mockProgramTopics, mockProgramValue);
    
    if (event.type === 'Payout') {
      console.log('Successfully decoded PayoutEvent!');
      console.log(`- Program ID: ${event.program_id}`);
      console.log(`- Recipient: ${event.recipient}`);
      console.log(`- Amount Paid: ${event.amount}`);
      console.log(`- Contract Remaining Balance: ${event.remaining_balance}`);
    }
  } catch (error: any) {
    console.error('Decoding failed:', error.message);
  }
}

main().catch(console.error);
