#pragma once

#include <cstdint>

// Called from onContextReady and after successful createStream (Step 12 inventory).
void paymentStreamsModuleOnContextReady(const char* persistenceDirUtf8);
void paymentStreamsModuleRecordStreamInventory(uint64_t vaultId, uint64_t streamId);
